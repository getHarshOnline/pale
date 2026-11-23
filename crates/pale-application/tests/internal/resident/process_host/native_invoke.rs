//! Exact process-host composition for one built-in root Invoke.

use super::*;
use crate::actions::NativeActionRegistry;
use crate::local_operation_start_fixture::{
    artifact_digest, checkpoint_authority, checkpoint_preimage_at_store, issue_observation,
    record_ref, schema_digest, signature, source_generation_input_copies_with, RecoveryParents,
};
use crate::resident::consultation::coordinator::ResidentConsultationCoordinator;
use crate::resident::consultation::service::task::{
    ResidentConsultationTaskAcceptanceFailure, ResidentConsultationTaskPoll,
    ResidentConsultationTaskRefusal,
};
use crate::resident::consultation::tests::{
    consultation_supervision_fixture_for_operation_in_store,
    consultation_supervision_fixture_for_provider_operation_in_store,
    consultation_supervision_fixture_for_self_contained_operation_in_store,
    prepare_collection_accumulator, recover_completed_collection, CollectionCommitPort,
};
use crate::resident::consultation::{
    ConsultationChildDispatchBatch, ConsultationChildJoinInput, ConsultationCollectionLimits,
    ConsultationSlotDisposition, ConsultationStopCause, ConsultationStopInput,
    ConsultationSuccessorCommitInput, ResidentConsultationPreparedResumeFailure,
    ResidentConsultationTaskOwnerAcceptanceFailure, ResidentConsultationTurnProgress,
};
use crate::resident::operation_dispatch::selection::tests::{
    provider::{safe_failure_at, Evidence, ResultPort},
    with_provider_selection_for_operation,
};
use crate::resident::operation_dispatch::{
    complete_prepared_checkpoint, record_native_operation_outcome,
    record_recovered_provider_operation_unknown, retain_provider_operation_dispatch,
    CompletedNativeOperation, NativeOperationInvokeFailure, NativeOperationOutcomeRecordingInput,
    NativeOperationRecordingFailure, NativeOperationTerminalRecordingInput, OperationAppendFailure,
    OperationAttemptError, OperationCheckpointInput,
};
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_native_recovery_fixtures, ProcessHostRestartFixture,
};
use crate::resident::operation_observation_custody::tests::{
    local_start, native_execution as custody,
};
use crate::resident::process_host::consult_child::*;
use crate::resident::process_host::consult_provider::*;
use crate::resident::process_host::native_invoke::*;
use crate::resident::process_host::operation_resolution::*;
use crate::resident::process_host::restart::*;
use crate::resident::process_host::service_assembly::retained_provider_invoke_tests::{
    expire_consult_child_before_dispatch, ConsultChildExpiryInputs, ExactExpiryClock,
};
use crate::resident::{
    recover_operation_control, AdmittedCompiledGeneration,
    DispatchedOperationCancellationRequestInput, LocalOperationCancellationRequestInput,
    LocalOperationHandleRecoveryInput, LocalOperationRecoveryContext,
    OperationCheckpointAncestryLimits, OperationControlRecoveryAdmission,
    OperationControlRecoveryInput, OperationObservationAuthorityStoreRejection,
    OperationObservationAuthorityStoreResolver, OperationObservationRecoveryLimits,
    OperationObservationSelection, RecoveredOperationControl,
};
use crate::resident_fixture::RecoveryResolver;
use crate::session_admission_fixture::{LocalStartContext, ProvingAuthority};
use pale_core::records::{
    CapabilityAttemptIdV1, CapabilityProvider, CapabilityResultAdmissionContextV1,
    CapabilitySuccessAdmissionContextV1, ControlCheckpointIdV1, ControlStateCheckpointWireV1,
    ControlTransactionSequenceV1, OperationActivePhaseV1, OperationObservationAdmissionContextV1,
    OperationObservationRevisionV1, OperationObservationStateV1, OperationObservationWireV1,
    ProviderBindingIdentityV1, ProviderBindingRefV1, SchemaDigest, SchemaId,
    SessionAdmissionLeaseWireV1, SignedResidentAdmissionContextV1, UnixTimeV1,
    CAPABILITY_RESULT_V1_SCHEMA_ID, CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorMembershipLimits, AccumulatorPreparationLimits, AmbientSessionAuthorityPort,
    AuthorityStoreCoordinate, ControlStoreError, OperationCustodyCommitOutcome,
    OperationCustodyIndexKey, OperationCustodyIndexQuery, OperationCustodyRecordKind,
    ProviderInvocationEvidenceFrontier, ProviderInvocationEvidenceSubject,
    ProviderInvocationObservationCoordinate, SessionLeaseBody, SessionLeaseBodyRead,
    SessionLeaseBodyRetention, SessionLeaseCustodyPort, WallClockSourceError, WallClockSourcePort,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::consult_child_fixture::{
    expect_consult_child_terminal_join, with_process_consult_invoke_child, ConsultChildFixtureKind,
    DeferredConsultChildExecutor,
};
use super::provider::with_provider_dispatch_inputs;

mod cold_provider;

struct SelectedStore {
    coordinate: AuthorityStoreCoordinate,
    resolutions: Cell<usize>,
    fail_at: Option<usize>,
}

impl SelectedStore {
    fn available(coordinate: AuthorityStoreCoordinate) -> Self {
        Self {
            coordinate,
            resolutions: Cell::new(0),
            fail_at: None,
        }
    }

    fn failing_once(coordinate: AuthorityStoreCoordinate, fail_at: usize) -> Self {
        Self {
            coordinate,
            resolutions: Cell::new(0),
            fail_at: Some(fail_at),
        }
    }
}

impl OperationObservationAuthorityStoreResolver for SelectedStore {
    fn resolve(
        &self,
        _: &pale_core::records::OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        let resolution = self.resolutions.get() + 1;
        self.resolutions.set(resolution);
        if self.fail_at == Some(resolution) {
            return Err(OperationObservationAuthorityStoreRejection);
        }
        Ok(self.coordinate.clone())
    }
}

struct IncreasingClock {
    next: AtomicUsize,
    reads: AtomicUsize,
    fail_at: Option<usize>,
}

impl IncreasingClock {
    fn new(fail_at: Option<usize>) -> Self {
        Self::starting_at(61, fail_at)
    }

    fn starting_at(first: u64, fail_at: Option<usize>) -> Self {
        Self {
            next: AtomicUsize::new(usize::try_from(first).expect("fixture time fits usize")),
            reads: AtomicUsize::new(0),
            fail_at,
        }
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl WallClockSourcePort for IncreasingClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let read = self.reads.fetch_add(1, Ordering::SeqCst) + 1;
        let value = self.next.fetch_add(1, Ordering::SeqCst);
        if self.fail_at == Some(read) {
            return Err(WallClockSourceError::from_source(std::io::Error::other(
                "modeled clock interruption",
            )));
        }
        Ok(UnixTimeV1::new(value as u64).expect("fixture time"))
    }
}

struct UnusedSessions;

impl SessionLeaseCustodyPort for UnusedSessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("native invocation does not renew a historical session")
    }

    fn read(
        &self,
        _: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        panic!("native invocation uses the already admitted session")
    }
}

struct RestartSessions(SessionLeaseBody);

impl SessionLeaseCustodyPort for RestartSessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("native restart must not retain or renew a session")
    }

    fn read(
        &self,
        query: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        assert_eq!(query.store(), self.0.store());
        assert_eq!(query.digest(), self.0.digest());
        Ok(Some(self.0.clone()))
    }
}

#[derive(Clone, Copy, Debug)]
enum NativeProcessCase {
    Success,
    HandlerFailure,
    StageSignerContinuation,
    StageResolverContinuation,
    StageCheckpointSignerContinuation,
    StageAttachmentContinuation,
    StageFinalizationMismatch,
    StageCommitFailure,
    StageCommitContinuation,
    StageAnchorContinuation,
    TerminalPreparationContinuation,
    TerminalCustodyContinuation,
    StagedRecoverySuccess,
    StagedRecoveryFailure,
    StagedRecoverySuccessAfterCancellation,
    StagedRecoveryFailureAfterCancellation,
    CurrentAuthorityRejection,
    AssociationRejection,
    BadStartEvidence,
    CancellationBeforeDispatch,
    InvocationBoundary,
}

fn modeled_signing_failure() -> pale_core::records::InvalidResidentRecordV1 {
    pale_core::records::InvalidResidentRecordV1::ContextualAdmission {
        record: "native continuation fixture",
        invariant: "modeled signer refusal",
    }
}

fn assert_terminal_lineage(
    completed: &CompletedNativeOperation,
    initial: &pale_core::records::ControlStateCheckpointV1,
) {
    let (terminal_ref, terminal) = completed.observation();
    assert_eq!(terminal_ref, &record_ref("process-native/terminal"));
    assert!(terminal.state().is_terminal());
    assert_eq!(
        terminal.predecessor_ref().as_ref(),
        Some(&record_ref("process-native/outcome-observation"))
    );
    let checkpoint = completed.custody().candidate().checkpoint();
    assert_eq!(
        checkpoint.successor_record().coordinate().record_ref(),
        &record_ref("process-native/terminal-checkpoint")
    );
    assert_eq!(
        checkpoint
            .predecessor_record()
            .expect("terminal append retains the staged checkpoint")
            .coordinate()
            .record_ref(),
        &record_ref("process-native/outcome-checkpoint")
    );
    assert_eq!(
        checkpoint.successor().domain_sequence().get(),
        initial.domain_sequence().get() + 4
    );
}

struct NativeCaseFaults {
    expected_preparations: usize,
    expected_commits: usize,
    expected_read_backs: usize,
    expected_anchor_advances: usize,
    expected_anchor_observations: usize,
    conflicts: &'static [usize],
    unknown_exact_commits: &'static [usize],
    unavailable_read_backs: &'static [usize],
    unavailable_advances: &'static [usize],
    clock_failure: Option<usize>,
}

impl NativeProcessCase {
    const fn faults(self) -> NativeCaseFaults {
        match self {
            Self::Success
            | Self::HandlerFailure
            | Self::StageSignerContinuation
            | Self::StageResolverContinuation => NativeCaseFaults {
                expected_preparations: 4,
                expected_commits: 4,
                expected_read_backs: 4,
                expected_anchor_advances: 4,
                expected_anchor_observations: 4,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: None,
            },
            Self::StageCheckpointSignerContinuation => NativeCaseFaults {
                expected_preparations: 5,
                expected_commits: 4,
                expected_read_backs: 4,
                expected_anchor_advances: 4,
                expected_anchor_observations: 4,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: None,
            },
            Self::StageAttachmentContinuation => NativeCaseFaults {
                expected_preparations: 5,
                expected_commits: 3,
                expected_read_backs: 3,
                expected_anchor_advances: 3,
                expected_anchor_observations: 3,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: None,
            },
            Self::StageFinalizationMismatch => NativeCaseFaults {
                expected_preparations: 3,
                expected_commits: 3,
                expected_read_backs: 3,
                expected_anchor_advances: 4,
                expected_anchor_observations: 3,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[3],
                clock_failure: None,
            },
            Self::StageCommitFailure => NativeCaseFaults {
                expected_preparations: 3,
                expected_commits: 3,
                expected_read_backs: 3,
                expected_anchor_advances: 2,
                expected_anchor_observations: 3,
                conflicts: &[3],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: None,
            },
            Self::StageCommitContinuation => NativeCaseFaults {
                expected_preparations: 4,
                expected_commits: 4,
                expected_read_backs: 6,
                expected_anchor_advances: 4,
                expected_anchor_observations: 4,
                conflicts: &[],
                unknown_exact_commits: &[3],
                unavailable_read_backs: &[4],
                unavailable_advances: &[],
                clock_failure: None,
            },
            Self::StageAnchorContinuation => NativeCaseFaults {
                expected_preparations: 4,
                expected_commits: 4,
                expected_read_backs: 4,
                expected_anchor_advances: 5,
                expected_anchor_observations: 4,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[3],
                clock_failure: None,
            },
            Self::TerminalPreparationContinuation => NativeCaseFaults {
                expected_preparations: 4,
                expected_commits: 4,
                expected_read_backs: 4,
                expected_anchor_advances: 4,
                expected_anchor_observations: 4,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: Some(7),
            },
            Self::StagedRecoverySuccess | Self::StagedRecoveryFailure => NativeCaseFaults {
                expected_preparations: 4,
                expected_commits: 4,
                expected_read_backs: 4,
                expected_anchor_advances: 4,
                expected_anchor_observations: 4,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: Some(7),
            },
            Self::StagedRecoverySuccessAfterCancellation
            | Self::StagedRecoveryFailureAfterCancellation => NativeCaseFaults {
                expected_preparations: 5,
                expected_commits: 5,
                expected_read_backs: 5,
                expected_anchor_advances: 5,
                expected_anchor_observations: 5,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: Some(7),
            },
            Self::TerminalCustodyContinuation => NativeCaseFaults {
                expected_preparations: 4,
                expected_commits: 4,
                expected_read_backs: 6,
                expected_anchor_advances: 4,
                expected_anchor_observations: 4,
                conflicts: &[],
                unknown_exact_commits: &[4],
                unavailable_read_backs: &[5],
                unavailable_advances: &[],
                clock_failure: None,
            },
            Self::CurrentAuthorityRejection
            | Self::AssociationRejection
            | Self::BadStartEvidence => NativeCaseFaults {
                expected_preparations: 0,
                expected_commits: 0,
                expected_read_backs: 0,
                expected_anchor_advances: 0,
                expected_anchor_observations: 0,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: None,
            },
            Self::CancellationBeforeDispatch => NativeCaseFaults {
                expected_preparations: 1,
                expected_commits: 1,
                expected_read_backs: 1,
                expected_anchor_advances: 1,
                expected_anchor_observations: 1,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: None,
            },
            Self::InvocationBoundary => NativeCaseFaults {
                expected_preparations: 2,
                expected_commits: 2,
                expected_read_backs: 2,
                expected_anchor_advances: 2,
                expected_anchor_observations: 2,
                conflicts: &[],
                unknown_exact_commits: &[],
                unavailable_read_backs: &[],
                unavailable_advances: &[],
                clock_failure: Some(3),
            },
        }
    }
}

fn run_native_case(case: NativeProcessCase) {
    let mut source_generations = source_generation_input_copies_with(|_| {}, |_| {}, 2).into_iter();
    let (compiled, materializations, profile) = source_generations
        .next()
        .expect("native fixture generation");
    let generation = AdmittedCompiledGeneration::admit(
        compiled,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .expect("compiled native fixture");
    let (host_compiled, host_materializations, _) = source_generations
        .next()
        .expect("retained native fixture generation");
    let host_generation = AdmittedCompiledGeneration::admit(
        host_compiled,
        host_materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .expect("retained native fixture");
    assert_eq!(
        generation.record().compiled_generation_digest(),
        host_generation.record().compiled_generation_digest()
    );
    local_start::with_issued_context(
        &generation,
        &profile,
        "process-native",
        |request| {
            let descriptor =
                pale_contracts::schema_descriptor("action-input.v1").expect("action-input schema");
            request.body.schema_id =
                SchemaId::parse(descriptor.schema_id).expect("schema identity");
            request.body.schema_sha256 =
                SchemaDigest::parse(descriptor.schema_sha256).expect("schema digest");
            request.body.payload = serde_json::json!({
                "kind": "path_inventory",
                "payload": {
                    "style": "posix_relative",
                    "paths": if matches!(
                        case,
                        NativeProcessCase::HandlerFailure | NativeProcessCase::StagedRecoveryFailure
                            | NativeProcessCase::StagedRecoveryFailureAfterCancellation
                    ) {
                        vec!["../token=private"]
                    } else {
                        vec!["src/./lib.rs"]
                    }
                }
            });
            request.body.payload_sha256 = pale_core::records::ArtifactDigest::hash_bytes(
                pale_core::canon::to_canonical_string(&request.body.payload)
                    .expect("canonical action input")
                    .as_bytes(),
            );
        },
        LocalStartContext::SelfContained,
        |operation, request, parents, initial_checkpoint, initial_record, initial_candidate| {
            let selected_store = initial_candidate.store().clone();
            let selector = selector_from_signing_authority(parents.signing);
            let registry = NativeActionRegistry::new().expect("sealed native registry");
            let attempt_id =
                CapabilityAttemptIdV1::parse("process-native-attempt").expect("attempt identity");
            let binding = request
                .local_request_binding()
                .expect("local request binding");
            let contracts = crate::session_admission_fixture::local_start_contract_set();
            let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
            let kind = pale_core::records::BoundedResidentTextV1::parse(
                CapabilityProvider::NativeRust.as_str(),
            )
            .expect("native implementation kind");
            let version = pale_core::records::BoundedResidentTextV1::parse(
                registry
                    .contract(pale_core::records::PATH_INVENTORY_ACTION_ID)
                    .expect("path inventory contract")
                    .version
                    .to_string(),
            )
            .expect("native implementation version");
            let admission = CapabilitySuccessAdmissionContextV1 {
                result: CapabilityResultAdmissionContextV1 {
                    expected_schema_sha256: &result_schema,
                    expected_request_binding: &binding,
                    expected_attempt_id: &attempt_id,
                    expected_implementation_kind: &kind,
                    expected_implementation_version: &version,
                    signing_authority: parents.signing,
                },
                contract_set_ref: binding.contract_set_ref(),
                contract_set: &contracts,
            };
            let (handle_ref, handle) = operation.handle();
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(1).expect("initial revision"),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            assert_eq!(
                initial_record.canonical_record_jcs().as_slice(),
                pale_core::canon::to_canonical_string(&initial)
                    .expect("initial observation encoding")
                    .as_bytes()
            );
            let observation = OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: parents.signing,
                },
                handle_ref,
                handle,
                predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
            };
            let mut checkpoint_fields = checkpoint_preimage_at_store(2, None, &selected_store);
            checkpoint_fields.checkpoint_domain_id =
                initial_checkpoint.checkpoint_domain_id().clone();
            checkpoint_fields.checkpoint_profile_ref =
                initial_checkpoint.checkpoint_profile_ref().clone();
            checkpoint_fields.checkpoint_profile_sha256 =
                initial_checkpoint.checkpoint_profile_sha256().clone();
            let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
            let checkpoint_input = |name: &str| OperationCheckpointInput {
                reference: record_ref(&format!("process-native/{name}-checkpoint")),
                id: ControlCheckpointIdV1::parse(format!("process-native-{name}"))
                    .expect("checkpoint identity"),
            };
            let faults = case.faults();
            let clock = IncreasingClock::new(faults.clock_failure);
            let resolver = if matches!(case, NativeProcessCase::StageResolverContinuation) {
                SelectedStore::failing_once(selected_store.clone(), 5)
            } else {
                SelectedStore::available(selected_store.clone())
            };
            let handle_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle_ref,
                    handle.canonical_operation_handle_sha256().as_str(),
                )
                .expect("handle coordinate"),
                handle,
            )
            .expect("handle record");
            let lease_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle.operation_lease_ref(),
                    parents.lease.operation_lease_digest().as_str(),
                )
                .expect("lease coordinate"),
                parents.lease,
            )
            .expect("lease record");
            custody::with_service_custody_faults(
                initial_checkpoint,
                Some(initial_candidate),
                Vec::new(),
                Some(lease_record),
                Vec::new(),
                faults.expected_preparations,
                faults.expected_commits,
                faults.expected_read_backs,
                faults.expected_anchor_advances,
                faults.expected_anchor_observations,
                faults.conflicts,
                faults.unknown_exact_commits,
                faults.unavailable_read_backs,
                faults.unavailable_advances,
                &[],
                &[],
                Vec::new(),
                false,
                Some(handle_record),
                Rc::new(RefCell::new(None)),
                |custody, anchor| {
                    let application_store = MemoryAuthorityStore::default();
                    let generation_digest =
                        generation.record().compiled_generation_digest().clone();
                    let mut application =
                        ready_application(&application_store, selected_store.clone(), 1);
                    application.fixture_retain_generation(host_generation);
                    let deployment = unproved_deployment_for_selector(
                        &selector,
                        selected_store.clone(),
                        application,
                    );
                    let host = ResidentProcessHost::new(
                        unproved_writer(
                            selector.principal_id.clone(),
                            vec![selected_store.clone()],
                        ),
                        vec![deployment],
                        NonZeroUsize::new(1).expect("one deployment"),
                        ResidentOperationHost::new(task_state()),
                    )
                    .expect("native process host");
                    assert!(host.deployments[0]
                        .application
                        .generation(&generation_digest)
                        .is_some());
                    let signs = Cell::new(0_usize);
                    let checkpoint_signs = Cell::new(0_usize);
                    let failed_stage_preimage = RefCell::new(None::<Vec<u8>>);
                    let failed_checkpoint_preimage = RefCell::new(None::<Vec<u8>>);
                    let foreign = owner_native_signing_authority(selector.principal_id.clone());
                    let current = parents.current;
                    let foreign_current = crate::resident::AmbientSessionAdmissionContext {
                        now: current.now,
                        subject: current.subject,
                        grant: current.grant,
                        current_principal_trust: current.current_principal_trust,
                        capability_revocations: current.capability_revocations,
                        deployment_binding_ref: current.deployment_binding_ref,
                        deployment_binding: current.deployment_binding,
                        deployment_revocations: current.deployment_revocations,
                        runtime_authority: current.runtime_authority,
                        session_revocations: current.session_revocations,
                        route_use: current.route_use,
                        generation: pale_core::records::SessionAdmissionLeaseContextV1 {
                            active_head: current.generation.active_head,
                            compiled_generation: current.generation.compiled_generation,
                            effective_profile: current.generation.effective_profile,
                            activation_fence: current.generation.activation_fence,
                            signed: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: current
                                    .generation
                                    .signed
                                    .expected_schema_sha256,
                                signing_authority: &foreign,
                            },
                        },
                    };
                    let selected_current =
                        if matches!(case, NativeProcessCase::CurrentAuthorityRejection) {
                            &foreign_current
                        } else {
                            current
                        };
                    let terminal_input = || NativeOperationTerminalRecordingInput {
                        result_ref: record_ref("process-native/result"),
                        diagnostic_ref: record_ref("process-native/diagnostic"),
                        terminal_ref: record_ref("process-native/terminal"),
                        terminal_checkpoint: checkpoint_input("terminal"),
                        clock: &clock,
                        resolve_store: &resolver,
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_signing,
                        signing: observation.signed,
                        maximum_diagnostic_bytes: 65_536,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            8, 20_000, 20_000, 8_192, 200_000,
                        )
                        .expect("accumulator limits"),
                    };
                    let session = parents.session.lease();
                    let sessions = RestartSessions(SessionLeaseBody::new(
                        selected_store.clone(),
                        session.session_admission_lease_digest().clone(),
                        session.schema_id().clone(),
                        session.schema_sha256().clone(),
                        pale_ports::ControlRecordBytes::new(
                            pale_core::canon::to_canonical_string(
                                &SessionAdmissionLeaseWireV1::from(session),
                            )
                            .expect("session wire")
                            .into_bytes(),
                        )
                        .expect("bounded session wire"),
                    ));
                    let mut historical = RecoveryResolver::default();
                    historical.insert_generation(parents.current.generation.compiled_generation);
                    historical.insert_active(parents.current.generation.active_head);
                    historical.insert_fence(parents.current.generation.activation_fence);
                    historical
                        .insert_effective_profile(parents.current.generation.effective_profile);
                    let expected_handle = crate::resident::record_projection::coordinate(
                        handle_ref,
                        handle.canonical_operation_handle_sha256().as_str(),
                    )
                    .expect("operation handle coordinate");
                    let observation_limits =
                        OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20)
                            .expect("observation bounds");
                    let checkpoint_limits = OperationCheckpointAncestryLimits::new(8, 1 << 20, 8)
                        .expect("checkpoint bounds");
                    let membership_limits = AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                        .expect("membership bounds");
                    let checkpoint_profile = crate::local_operation_start_fixture::profile();
                    let recovery = || OperationControlRecoveryInput {
                        operation: LocalOperationHandleRecoveryInput {
                            context: LocalOperationRecoveryContext {
                                store: &selected_store,
                                profile: &checkpoint_profile,
                                handle_signing_authority: parents.signing,
                                checkpoint_authority: &checkpoint_signing,
                                historical: &historical,
                                maximum_record_bytes: 1 << 20,
                                maximum_operation_lease_bytes: 1 << 20,
                                maximum_session_lease_bytes: 1 << 20,
                                membership_limits,
                                observation_limits,
                                checkpoint_limits,
                            },
                            expected_handle: &expected_handle,
                            maximum_parent_depth: 0,
                        },
                        observation: OperationObservationSelection::SelectedHead,
                        observation_limits,
                        checkpoint_limits,
                        membership_limits,
                    };
                    if matches!(case, NativeProcessCase::CancellationBeforeDispatch) {
                        let cancellation_request =
                            super::a2a_cancel_task::cancellation_request(request, handle);
                        let cancellation_request_ref =
                            record_ref("process-native/pre-dispatch-cancellation-request");
                        let cancellation_observation_ref =
                            record_ref("process-native/pre-dispatch-cancellation-requested");
                        let mut admit_request =
                            |reference: &pale_core::records::ResidentRecordRefV1,
                             wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                                assert_eq!(
                                    reference,
                                    operation.request_coordinate().record_ref()
                                );
                                assert_eq!(
                                    pale_core::canon::to_canonical_string(&wire)
                                        .expect("request wire encoding"),
                                    pale_core::canon::to_canonical_string(request)
                                        .expect("expected request wire encoding")
                                );
                                Ok(request.clone())
                            };
                        let mut admit_handle_authority =
                            |_: &pale_core::records::OperationHandlePreimageV1,
                             parent: Option<&pale_core::records::OperationHandleV1>| {
                                assert!(parent.is_none());
                                Ok(())
                            };
                        let mut verify_handle_signature =
                            |_: &[u8],
                             _: &pale_core::records::OperationHandlePreimageV1,
                             _: &pale_core::records::OperationHandleDigest,
                             _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
                        let mut admit_checkpoint =
                            |_: &pale_core::records::ResidentRecordRefV1,
                             wire: pale_core::records::ControlStateCheckpointWireV1| {
                                pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                    &checkpoint_signing,
                                    |_, _, _, _| Ok(()),
                                )
                            };
                        let mut resolve_observation_signer =
                            |_: &pale_core::records::ResidentRecordRefV1,
                             _: &pale_core::records::OperationObservationWireV1| {
                                Ok(parents.signing.clone())
                            };
                        let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
                        let mut admit_rotation =
                            |_: &pale_core::records::ResidentRecordRefV1,
                             _: &pale_core::records::ArtifactDigest,
                             _: &pale_core::records::ResidentRecordRefV1,
                             _: &pale_core::records::ArtifactDigest| {
                                Ok(())
                            };
                        let mut verify_observation_signature =
                            |_: &[u8],
                             _: &pale_core::records::OperationObservationPreimageV1,
                             _: &pale_core::records::OperationObservationDigest,
                             _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
                        let mut verify_checkpoint_signature =
                            |_: &[u8],
                             _: &pale_core::records::ControlStateCheckpointPreimageV1,
                             _: &pale_core::records::ControlStateCheckpointDigest,
                             _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
                        let recovered = recover_operation_control(
                            recovery(),
                            custody,
                            &sessions,
                            anchor,
                            OperationControlRecoveryAdmission {
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
                        .expect("pre-dispatch cancellation recovery");
                        let authorized = recovered
                            .authorize_cancellation(
                                &cancellation_request,
                                UnixTimeV1::new(70).expect("cancellation time"),
                                |_| Ok(()),
                            )
                            .expect("pre-dispatch cancellation authority");
                        let prepared = authorized
                            .prepare_local_request(
                                LocalOperationCancellationRequestInput {
                                    request_ref: cancellation_request_ref,
                                    observation_ref: cancellation_observation_ref,
                                    signing: observation.signed,
                                },
                                |_, _, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                            )
                            .expect("pre-dispatch cancellation preparation");
                        let prior = prepared.predecessor_checkpoint();
                        let prepared_checkpoint = prepared
                            .prepare_checkpoint(
                                AccumulatorPreparationLimits::new(
                                    8, 20_000, 20_000, 8_192, 200_000,
                                )
                                .expect("accumulator limits"),
                                &resolver,
                                custody,
                            )
                            .expect("pre-dispatch cancellation checkpoint");
                        complete_prepared_checkpoint(
                            prepared_checkpoint,
                            prior,
                            checkpoint_input("pre-dispatch-cancellation"),
                            UnixTimeV1::new(70).expect("cancellation checkpoint time"),
                            &checkpoint_fields.schema_sha256,
                            &checkpoint_signing,
                            custody,
                            anchor,
                            |_, _, _| Ok(signature()),
                        )
                        .expect("pre-dispatch cancellation custody");
                    }
                    let mut admit_request =
                        |reference: &pale_core::records::ResidentRecordRefV1,
                         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                            assert_eq!(reference, operation.request_coordinate().record_ref());
                            assert_eq!(
                                pale_core::canon::to_canonical_string(&wire)
                                    .expect("request wire encoding"),
                                pale_core::canon::to_canonical_string(request)
                                    .expect("expected request wire encoding")
                            );
                            Ok(request.clone())
                        };
                    let mut admit_handle_authority =
                        |_: &pale_core::records::OperationHandlePreimageV1,
                         parent: Option<&pale_core::records::OperationHandleV1>| {
                            assert!(parent.is_none());
                            Ok(())
                        };
                    let mut verify_handle_signature =
                        |_: &[u8],
                         _: &pale_core::records::OperationHandlePreimageV1,
                         _: &pale_core::records::OperationHandleDigest,
                         _: &pale_core::records::CanonicalSignatureV1| {
                            Ok(())
                        };
                    let mut admit_checkpoint =
                        |_: &pale_core::records::ResidentRecordRefV1,
                         wire: pale_core::records::ControlStateCheckpointWireV1| {
                            pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                &checkpoint_signing,
                                |_, _, _, _| Ok(()),
                            )
                        };
                    let mut resolve_observation_signer =
                        |_: &pale_core::records::ResidentRecordRefV1,
                         _: &pale_core::records::OperationObservationWireV1| {
                            Ok(parents.signing.clone())
                        };
                    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
                    let mut admit_recovery_rotation =
                        |_: &pale_core::records::ResidentRecordRefV1,
                         _: &pale_core::records::ArtifactDigest,
                         _: &pale_core::records::ResidentRecordRefV1,
                         _: &pale_core::records::ArtifactDigest| { Ok(()) };
                    let mut verify_observation_signature =
                        |_: &[u8],
                         _: &pale_core::records::OperationObservationPreimageV1,
                         _: &pale_core::records::OperationObservationDigest,
                         _: &pale_core::records::CanonicalSignatureV1| {
                            Ok(())
                        };
                    let mut verify_checkpoint_signature =
                        |_: &[u8],
                         _: &pale_core::records::ControlStateCheckpointPreimageV1,
                         _: &pale_core::records::ControlStateCheckpointDigest,
                         _: &pale_core::records::CanonicalSignatureV1| {
                            Ok(())
                        };
                    let unlike_request = if matches!(case, NativeProcessCase::BadStartEvidence) {
                        let mut preimage = request.to_preimage();
                        preimage.idempotency_key =
                            pale_core::records::RequestIdempotencyKeyV1::parse(
                                "process-native-unlike-start",
                            )
                            .expect("unlike idempotency key");
                        Some(crate::effect_case_lineage_fixture::request(&preimage))
                    } else {
                        None
                    };
                    let classification_request = unlike_request.as_ref().unwrap_or(request);
                    let recovered = host.classify_started_operation(
                        &selector,
                        operation,
                        classification_request,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions: &sessions,
                            anchor,
                        },
                        native_recovery_input(recovery()),
                        OperationControlRecoveryAdmission {
                            admit_request: &mut admit_request,
                            admit_handle_authority: &mut admit_handle_authority,
                            verify_handle_signature: &mut verify_handle_signature,
                            admit_checkpoint: &mut admit_checkpoint,
                            resolve_observation_signer: &mut resolve_observation_signer,
                            admit_observation_state: &mut admit_observation_state,
                            admit_rotation: &mut admit_recovery_rotation,
                            verify_observation_signature: &mut verify_observation_signature,
                            verify_checkpoint_signature: &mut verify_checkpoint_signature,
                        },
                    );
                    if matches!(case, NativeProcessCase::BadStartEvidence) {
                        match recovered {
                            Err(reason) => assert!(
                                matches!(
                                    reason,
                                    ResidentProcessOperationRestartError::StartEvidence
                                        | ResidentProcessOperationRestartError::StartEvidenceRead(
                                            crate::resident::operation_custody::OperationRecoveryError::RequestConflict
                                        )
                                ),
                                "unexpected unlike-start rejection: {reason:?}"
                            ),
                            Ok(_) => panic!("unlike request must not classify the issued start"),
                        }
                        assert_eq!(clock.reads(), 0);
                        assert_eq!(signs.get(), 0);
                        assert_eq!(checkpoint_signs.get(), 0);
                        return;
                    }
                    let recovered = recovered.expect("fresh start read-back and classification");
                    if matches!(case, NativeProcessCase::CancellationBeforeDispatch) {
                        let ResidentProcessOperationRestartDisposition::LocalCancellationRequested(
                            cancelled,
                        ) = recovered
                        else {
                            panic!("pre-dispatch cancellation must win reclassification")
                        };
                        assert!(matches!(
                            cancelled
                                .start
                                .control
                                .observation()
                                .selected_predecessor()
                                .expect("cancellation predecessor")
                                .1
                                .state(),
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving
                            }
                        ));
                        assert_eq!(clock.reads(), 0);
                        assert_eq!(signs.get(), 0);
                        assert_eq!(checkpoint_signs.get(), 0);
                        return;
                    }
                    let ResidentProcessOperationRestartDisposition::OperationResolution(recovered) =
                        recovered
                    else {
                        panic!("fresh self-contained Invoke remains resolving")
                    };
                    let invoke_signing = if matches!(case, NativeProcessCase::AssociationRejection)
                    {
                        SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: &foreign,
                        }
                    } else {
                        observation.signed
                    };
                    let mut rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
                    let mut sign_observation = |bytes: &[u8], _: &_, _: &_| {
                        let sign = signs.get() + 1;
                        signs.set(sign);
                        if matches!(case, NativeProcessCase::StageSignerContinuation) && sign == 3 {
                            *failed_stage_preimage.borrow_mut() = Some(bytes.to_vec());
                            return Err(modeled_signing_failure());
                        }
                        Ok(signature())
                    };
                    let mut sign_checkpoint = |bytes: &[u8], _: &_, _: &_| {
                        let sign = checkpoint_signs.get() + 1;
                        checkpoint_signs.set(sign);
                        if matches!(
                            case,
                            NativeProcessCase::StageCheckpointSignerContinuation
                                | NativeProcessCase::StageAttachmentContinuation
                        ) && sign == 3
                        {
                            *failed_checkpoint_preimage.borrow_mut() = Some(bytes.to_vec());
                            return Err(modeled_signing_failure());
                        }
                        Ok(signature())
                    };
                    let mut sign_result = |_: &[u8], _: &_, _: &_| Ok(signature());
                    let mut authority = ProvingAuthority;
                    let selected = host.invoke_self_contained_native(
                        &recovered,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions: &sessions,
                            anchor,
                        },
                        ResidentProcessOperationResolutionInput {
                            session: parents.session,
                            current: selected_current,
                            signing: invoke_signing,
                            selection_ref: record_ref("process-native/selection"),
                            observation_ref: record_ref("process-native/preparation"),
                            maximum_selection_bytes: 65_536,
                            maximum_record_bytes: 65_536,
                        },
                        ResidentProcessNativeInvokeBasis {
                            registry: &registry,
                            attempt_id: &attempt_id,
                            result_admission: admission,
                            checkpoint_schema: &checkpoint_fields.schema_sha256,
                            checkpoint_authority: &checkpoint_signing,
                            preparation_checkpoint: checkpoint_input("prepared"),
                            dispatch_checkpoint: checkpoint_input("dispatched"),
                            outcome_checkpoint: checkpoint_input("outcome"),
                            terminal_checkpoint: checkpoint_input("terminal"),
                            dispatch_ref: record_ref("process-native/dispatched"),
                            outcome_ref: record_ref("process-native/outcome"),
                            outcome_observation_ref: record_ref(
                                "process-native/outcome-observation",
                            ),
                            result_ref: record_ref("process-native/result"),
                            diagnostic_ref: record_ref("process-native/diagnostic"),
                            terminal_ref: record_ref("process-native/terminal"),
                            clock: &clock,
                            resolve_store: &resolver,
                            maximum_diagnostic_bytes: 65_536,
                            maximum_outcome_bytes: 65_536,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                8, 20_000, 20_000, 8_192, 200_000,
                            )
                            .expect("accumulator limits"),
                        },
                        &mut authority,
                        &mut rotation,
                        &mut sign_observation,
                        &mut sign_checkpoint,
                        &mut sign_result,
                    );
                    if matches!(case, NativeProcessCase::CurrentAuthorityRejection) {
                        assert!(matches!(
                            selected,
                            Err(ResidentProcessSelfContainedNativeInvokeFailure::Resolution(
                                ResidentProcessOperationResolutionFailure::Selection(
                                    ResidentProcessHostSelectionError::OperationResolutionCurrentAuthority
                                )
                            ))
                        ));
                        assert_eq!(clock.reads(), 0);
                        assert_eq!(signs.get(), 0);
                        return;
                    }
                    if matches!(case, NativeProcessCase::AssociationRejection) {
                        assert!(matches!(
                            selected,
                            Err(ResidentProcessSelfContainedNativeInvokeFailure::Resolution(
                                ResidentProcessOperationResolutionFailure::Selection(
                                    ResidentProcessHostSelectionError::OperationResolutionSigningAuthority
                                )
                            ))
                        ));
                        assert_eq!(clock.reads(), 0);
                        assert_eq!(signs.get(), 0);
                        return;
                    }
                    let result = selected.map_err(|reason| match reason {
                        ResidentProcessSelfContainedNativeInvokeFailure::Native(reason) => reason,
                        ResidentProcessSelfContainedNativeInvokeFailure::Resolution(reason) => {
                            panic!("native resolution failed after protected classification: {reason:?}")
                        }
                    });
                    if let Err(NativeOperationInvokeFailure::Admission(reason)) = &result {
                        panic!(
                            "native admission after {} clock reads: {reason:?}",
                            clock.reads()
                        );
                    }
                    match case {
                        NativeProcessCase::Success => {
                            let completed = result.expect("successful native invocation");
                            assert!(completed.result().is_ok());
                            assert_terminal_lineage(&completed, initial_checkpoint);
                        }
                        NativeProcessCase::HandlerFailure => {
                            let completed =
                                result.expect("failed handler retained terminal evidence");
                            assert!(completed.result().is_err());
                            assert_terminal_lineage(&completed, initial_checkpoint);
                        }
                        NativeProcessCase::StageSignerContinuation => {
                            let Err(NativeOperationInvokeFailure::Recording(
                                NativeOperationRecordingFailure::StagePreparation {
                                    continuation,
                                    ..
                                },
                            )) = result
                            else {
                                panic!("expected exact failed stage preparation")
                            };
                            let clock_reads = clock.reads();
                            assert_eq!(signs.get(), 3);
                            let foreign_store = store(
                                "process-native-foreign",
                                "process-native-foreign-incarnation",
                            );
                            let rejected = host
                                .continue_native_stage_preparation(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &foreign_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    *continuation,
                                    observation.signed,
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| panic!("foreign store must not sign"),
                                )
                                .expect_err("foreign store is rejected");
                            let (continuation, reason) = rejected.into_parts();
                            assert!(matches!(
                                reason,
                                ResidentProcessHostSelectionError::AccessStoreAssociation
                            ));
                            let mut foreign_selector = selector.clone();
                            foreign_selector.runtime_authority_ref =
                                record_ref("process-native/foreign-runtime-authority");
                            let rejected = host
                                .continue_native_stage_preparation(
                                    &foreign_selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    continuation,
                                    observation.signed,
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| panic!("foreign deployment must not sign"),
                                )
                                .expect_err("foreign deployment is rejected");
                            let (continuation, _) = rejected.into_parts();
                            let rejected = host
                                .continue_native_stage_preparation(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    continuation,
                                    SignedResidentAdmissionContextV1 {
                                        expected_schema_sha256: &observation_schema,
                                        signing_authority: &foreign,
                                    },
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| panic!("foreign signer must not sign"),
                                )
                                .expect_err("foreign signer is rejected");
                            let (continuation, reason) = rejected.into_parts();
                            assert!(matches!(
                                reason,
                                ResidentProcessHostSelectionError::NativeSigningAuthority
                            ));
                            assert_eq!(clock.reads(), clock_reads);
                            assert_eq!(signs.get(), 3);
                            let prepared = host
                                .continue_native_stage_preparation(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    continuation,
                                    observation.signed,
                                    |_, _, _, _| Ok(()),
                                    |bytes, _, _| {
                                        signs.set(signs.get() + 1);
                                        assert_eq!(
                                            Some(bytes),
                                            failed_stage_preimage.borrow().as_deref(),
                                            "stage retry must preserve refs and sampled time"
                                        );
                                        Ok(signature())
                                    },
                                )
                                .expect("stage retry association")
                                .expect("stage retry");
                            assert_eq!(clock.reads(), clock_reads);
                            let finalization = host
                                .continue_native_prepared_stage(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    prepared,
                                    &resolver,
                                    |_, _, _| {
                                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                                        Ok(signature())
                                    },
                                )
                                .expect("prepared stage association")
                                .expect("prepared stage custody");
                            let staged = host
                                .admit_native_stage(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    finalization,
                                )
                                .expect("stage admission association")
                                .expect("exact stage admission");
                            let completed = staged
                                .record_terminal(
                                    terminal_input(),
                                    custody,
                                    anchor,
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        signs.set(signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| {
                                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| Ok(signature()),
                                )
                                .expect("terminal after stage retry");
                            assert!(completed.result().is_ok());
                            assert_eq!(signs.get(), 5);
                        }
                        NativeProcessCase::StageResolverContinuation
                        | NativeProcessCase::StageCheckpointSignerContinuation => {
                            let continuation = match result {
                                Err(NativeOperationInvokeFailure::Recording(
                                    NativeOperationRecordingFailure::PreparedStage {
                                        continuation,
                                        ..
                                    },
                                )) => continuation,
                                other => panic!(
                                    "expected exact prepared stage for {case:?} after {} resolutions, got {other:?}",
                                    resolver.resolutions.get()
                                ),
                            };
                            let clock_reads = clock.reads();
                            let observation_signs = signs.get();
                            let prepared = *continuation;
                            let finalization = host
                                .continue_native_prepared_stage(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    prepared,
                                    &resolver,
                                    |bytes, _, _| {
                                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                                        if matches!(
                                            case,
                                            NativeProcessCase::StageCheckpointSignerContinuation
                                        ) {
                                            assert_eq!(
                                                Some(bytes),
                                                failed_checkpoint_preimage
                                                    .borrow()
                                                    .as_deref(),
                                                "checkpoint retry must preserve its ref, id, and time"
                                            );
                                        }
                                        Ok(signature())
                                    },
                                )
                                .expect("prepared stage association")
                                .expect("prepared stage retry");
                            assert_eq!(clock.reads(), clock_reads);
                            assert_eq!(signs.get(), observation_signs);
                            let staged = host
                                .admit_native_stage(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    finalization,
                                )
                                .expect("stage admission association")
                                .expect("exact stage admission");
                            let completed = staged
                                .record_terminal(
                                    terminal_input(),
                                    custody,
                                    anchor,
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        signs.set(signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| {
                                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| Ok(signature()),
                                )
                                .expect("terminal after prepared-stage retry");
                            assert!(completed.result().is_ok());
                        }
                        NativeProcessCase::StageAttachmentContinuation => {
                            let Err(NativeOperationInvokeFailure::Recording(
                                NativeOperationRecordingFailure::PreparedStage {
                                    continuation, ..
                                },
                            )) = result
                            else {
                                panic!("expected exact prepared stage before attachment refusal")
                            };
                            let failure = match continuation
                                .continue_custody_with_checkpoint_attachment(
                                    &resolver,
                                    custody,
                                    anchor,
                                    |_, _, _| {
                                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| {
                                        Err(crate::resident::OperationObservationCheckpointConstructionRejection.into())
                                    },
                                ) {
                                Err(failure) => failure,
                                Ok(_) => panic!("modeled attachment refusal must retain custody"),
                            };
                            let NativeOperationRecordingFailure::IssuedStage {
                                continuation, ..
                            } = failure
                            else {
                                panic!("attachment refusal must retain the issued checkpoint")
                            };
                            let checkpoint_sign_count = checkpoint_signs.get();
                            assert_eq!(checkpoint_sign_count, 4);
                            let issued_retry = host
                                .continue_native_issued_stage(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    *continuation,
                                    &resolver,
                                )
                                .expect("issued checkpoint association");
                            let finalization = match issued_retry {
                                Ok(finalization) => finalization,
                                Err(NativeOperationRecordingFailure::IssuedStage {
                                    failure,
                                    ..
                                }) => panic!("issued checkpoint attachment retry: {failure:?}"),
                                Err(failure) => {
                                    panic!("unexpected issued checkpoint phase: {failure:?}")
                                }
                            };
                            assert_eq!(
                                checkpoint_signs.get(),
                                checkpoint_sign_count,
                                "issued continuation must not call a checkpoint signer"
                            );
                            let _staged = host
                                .admit_native_stage(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    finalization,
                                )
                                .expect("stage admission association")
                                .expect("exact stage admission after attachment retry");
                        }
                        NativeProcessCase::StageCommitFailure => {
                            assert!(matches!(
                                result,
                                Err(NativeOperationInvokeFailure::Recording(
                                    NativeOperationRecordingFailure::StagingCommit(_)
                                ))
                            ));
                        }
                        NativeProcessCase::StageCommitContinuation => {
                            let Err(NativeOperationInvokeFailure::Recording(
                                NativeOperationRecordingFailure::StagingCommit(failure),
                            )) = result
                            else {
                                panic!("expected retained staging commit")
                            };
                            let pending = match host
                                .reconcile_native_stage_commit(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    *failure,
                                )
                                .expect("staging commit association")
                            {
                                Ok(pending) => pending,
                                Err(unavailable) => host
                                    .reconcile_native_stage_commit(
                                        &selector,
                                        ResidentDeploymentOperationAccess {
                                            store: &selected_store,
                                            custody,
                                            sessions: &UnusedSessions,
                                            anchor,
                                        },
                                        unavailable,
                                    )
                                    .expect("staging read-back association")
                                    .expect("exact staged successor after unavailable read-back"),
                            };
                            let finalization = host
                                .continue_native_stage_anchor(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    pending,
                                )
                                .expect("staging anchor association")
                                .expect("staged successor anchor");
                            let staged = host
                                .admit_native_stage(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    finalization,
                                )
                                .expect("stage admission association")
                                .expect("exact stage finalization");
                            let completed = staged
                                .record_terminal(
                                    terminal_input(),
                                    custody,
                                    anchor,
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        signs.set(signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| Ok(signature()),
                                    |_, _, _| Ok(signature()),
                                )
                                .expect("terminal continuation");
                            assert!(completed.result().is_ok());
                        }
                        NativeProcessCase::StageFinalizationMismatch => {
                            let Err(NativeOperationInvokeFailure::Recording(
                                NativeOperationRecordingFailure::StagingAnchor(pending),
                            )) = result
                            else {
                                panic!("expected retained staging anchor")
                            };
                            let finalization = host
                                .continue_native_stage_anchor(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    *pending,
                                )
                                .expect("staging anchor association")
                                .expect("staged successor anchor retry");
                            let unlike_checkpoint_ref =
                                record_ref("process-native/unlike-finalization-checkpoint");
                            let unlike_checkpoint = crate::resident::record_projection::coordinate(
                                &unlike_checkpoint_ref,
                                finalization
                                    .checkpoint()
                                    .1
                                    .canonical_control_checkpoint_sha256()
                                    .as_str(),
                            )
                            .expect("unlike checkpoint coordinate");
                            let before = (clock.reads(), signs.get(), checkpoint_signs.get());
                            let rejected = finalization
                                .finish_against_retained_checkpoint(&unlike_checkpoint)
                                .map_err(|failure| failure.into_reason());
                            assert!(matches!(rejected, Err(OperationAttemptError::Association)));
                            assert_eq!(
                                (clock.reads(), signs.get(), checkpoint_signs.get()),
                                before,
                                "finalization mismatch must not perform another effect"
                            );
                        }
                        NativeProcessCase::StageAnchorContinuation => {
                            let Err(NativeOperationInvokeFailure::Recording(
                                NativeOperationRecordingFailure::StagingAnchor(pending),
                            )) = result
                            else {
                                panic!("expected retained staging anchor")
                            };
                            let finalization = host
                                .continue_native_stage_anchor(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    *pending,
                                )
                                .expect("staging anchor association")
                                .expect("staged successor anchor retry");
                            let staged = host
                                .admit_native_stage(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &UnusedSessions,
                                        anchor,
                                    },
                                    finalization,
                                )
                                .expect("stage admission association")
                                .expect("exact stage finalization");
                            let completed = staged
                                .record_terminal(
                                    terminal_input(),
                                    custody,
                                    anchor,
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        signs.set(signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| Ok(signature()),
                                    |_, _, _| Ok(signature()),
                                )
                                .expect("terminal continuation");
                            assert!(completed.result().is_ok());
                        }
                        NativeProcessCase::TerminalPreparationContinuation => {
                            let Err(NativeOperationInvokeFailure::Recording(
                                NativeOperationRecordingFailure::TerminalPreparation {
                                    staged, ..
                                },
                            )) = result
                            else {
                                panic!("expected retained terminal preparation")
                            };
                            let completed = staged
                                .record_terminal(
                                    terminal_input(),
                                    custody,
                                    anchor,
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        signs.set(signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| Ok(signature()),
                                    |_, _, _| Ok(signature()),
                                )
                                .expect("terminal preparation continuation");
                            assert!(completed.result().is_ok());
                        }
                        NativeProcessCase::StagedRecoverySuccess
                        | NativeProcessCase::StagedRecoveryFailure
                        | NativeProcessCase::StagedRecoverySuccessAfterCancellation
                        | NativeProcessCase::StagedRecoveryFailureAfterCancellation => {
                            let Err(NativeOperationInvokeFailure::Recording(
                                NativeOperationRecordingFailure::TerminalPreparation {
                                    staged, ..
                                },
                            )) = result
                            else {
                                panic!("expected a committed staged native outcome")
                            };
                            drop(staged);

                            let session = parents.session.lease();
                            let sessions = RestartSessions(SessionLeaseBody::new(
                                selected_store.clone(),
                                session.session_admission_lease_digest().clone(),
                                session.schema_id().clone(),
                                session.schema_sha256().clone(),
                                pale_ports::ControlRecordBytes::new(
                                    pale_core::canon::to_canonical_string(
                                        &SessionAdmissionLeaseWireV1::from(session),
                                    )
                                    .expect("session wire")
                                    .into_bytes(),
                                )
                                .expect("bounded session wire"),
                            ));
                            let mut historical = RecoveryResolver::default();
                            historical
                                .insert_generation(parents.current.generation.compiled_generation);
                            historical.insert_active(parents.current.generation.active_head);
                            historical.insert_fence(parents.current.generation.activation_fence);
                            historical.insert_effective_profile(
                                parents.current.generation.effective_profile,
                            );
                            let expected_handle = crate::resident::record_projection::coordinate(
                                handle_ref,
                                handle.canonical_operation_handle_sha256().as_str(),
                            )
                            .expect("operation handle coordinate");
                            let observation_limits =
                                OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20)
                                    .expect("observation bounds");
                            let checkpoint_limits =
                                OperationCheckpointAncestryLimits::new(8, 1 << 20, 8)
                                    .expect("checkpoint bounds");
                            let membership_limits =
                                AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                                    .expect("membership bounds");
                            let checkpoint_profile =
                                crate::local_operation_start_fixture::profile();
                            let recovery = || OperationControlRecoveryInput {
                                operation: LocalOperationHandleRecoveryInput {
                                    context: LocalOperationRecoveryContext {
                                        store: &selected_store,
                                        profile: &checkpoint_profile,
                                        handle_signing_authority: parents.signing,
                                        checkpoint_authority: &checkpoint_signing,
                                        historical: &historical,
                                        maximum_record_bytes: 1 << 20,
                                        maximum_operation_lease_bytes: 1 << 20,
                                        maximum_session_lease_bytes: 1 << 20,
                                        membership_limits,
                                        observation_limits,
                                        checkpoint_limits,
                                    },
                                    expected_handle: &expected_handle,
                                    maximum_parent_depth: 0,
                                },
                                observation: OperationObservationSelection::SelectedHead,
                                observation_limits,
                                checkpoint_limits,
                                membership_limits,
                            };
                            let cancellation_request = matches!(
                                case,
                                NativeProcessCase::StagedRecoverySuccessAfterCancellation
                                    | NativeProcessCase::StagedRecoveryFailureAfterCancellation
                            )
                            .then(|| super::a2a_cancel_task::cancellation_request(request, handle));
                            let cancellation_request_ref =
                                record_ref("process-native/recovery-cancellation-request");
                            let cancellation_observation_ref =
                                record_ref("process-native/recovery-cancellation-requested");

                            if let Some(cancellation_request) = cancellation_request.as_ref() {
                                let mut admit_request = |reference: &pale_core::records::ResidentRecordRefV1,
                                                         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                                    assert_eq!(reference, operation.request_coordinate().record_ref());
                                    assert_eq!(
                                        pale_core::canon::to_canonical_string(&wire)
                                            .expect("request wire encoding"),
                                        pale_core::canon::to_canonical_string(request)
                                            .expect("expected request wire encoding")
                                    );
                                    Ok(request.clone())
                                };
                                let mut admit_handle_authority =
                                    |_: &pale_core::records::OperationHandlePreimageV1,
                                     parent: Option<&pale_core::records::OperationHandleV1>| {
                                        assert!(parent.is_none());
                                        Ok(())
                                    };
                                let mut verify_handle_signature = |_: &[u8],
                                                                   _: &pale_core::records::OperationHandlePreimageV1,
                                                                   _: &pale_core::records::OperationHandleDigest,
                                                                   _: &pale_core::records::CanonicalSignatureV1| {
                                    Ok(())
                                };
                                let mut admit_checkpoint =
                                    |_: &pale_core::records::ResidentRecordRefV1,
                                     wire: pale_core::records::ControlStateCheckpointWireV1| {
                                        pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                            wire,
                                            &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                            &checkpoint_signing,
                                            |_, _, _, _| Ok(()),
                                        )
                                    };
                                let mut resolve_observation_signer =
                                    |_: &pale_core::records::ResidentRecordRefV1,
                                     _: &pale_core::records::OperationObservationWireV1| {
                                        Ok(parents.signing.clone())
                                    };
                                let mut admit_observation_state =
                                    |_: &OperationObservationStateV1| Ok(());
                                let mut admit_rotation =
                                    |_: &pale_core::records::ResidentRecordRefV1,
                                     _: &pale_core::records::ArtifactDigest,
                                     _: &pale_core::records::ResidentRecordRefV1,
                                     _: &pale_core::records::ArtifactDigest| { Ok(()) };
                                let mut verify_observation_signature =
                                    |_: &[u8],
                                     _: &pale_core::records::OperationObservationPreimageV1,
                                     _: &pale_core::records::OperationObservationDigest,
                                     _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
                                let mut verify_checkpoint_signature =
                                    |_: &[u8],
                                     _: &pale_core::records::ControlStateCheckpointPreimageV1,
                                     _: &pale_core::records::ControlStateCheckpointDigest,
                                     _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
                                let recovered = recover_operation_control(
                                    recovery(),
                                    custody,
                                    &sessions,
                                    anchor,
                                    OperationControlRecoveryAdmission {
                                        admit_request: &mut admit_request,
                                        admit_handle_authority: &mut admit_handle_authority,
                                        verify_handle_signature: &mut verify_handle_signature,
                                        admit_checkpoint: &mut admit_checkpoint,
                                        resolve_observation_signer: &mut resolve_observation_signer,
                                        admit_observation_state: &mut admit_observation_state,
                                        admit_rotation: &mut admit_rotation,
                                        verify_observation_signature:
                                            &mut verify_observation_signature,
                                        verify_checkpoint_signature:
                                            &mut verify_checkpoint_signature,
                                    },
                                )
                                .expect("staged outcome recovery before cancellation");
                                let authorized = recovered
                                    .authorize_cancellation(
                                        cancellation_request,
                                        UnixTimeV1::new(70).expect("cancellation time"),
                                        |_| Ok(()),
                                    )
                                    .expect("staged outcome cancellation authority");
                                let prepared = authorized
                                    .prepare_dispatched_request(
                                        DispatchedOperationCancellationRequestInput {
                                            request_ref: cancellation_request_ref.clone(),
                                            observation_ref: cancellation_observation_ref.clone(),
                                            signing: observation.signed,
                                        },
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| Ok(signature()),
                                    )
                                    .expect("staged outcome cancellation preparation");
                                let prior = prepared.predecessor_checkpoint();
                                let prepared_checkpoint = prepared
                                    .prepare_checkpoint(
                                        AccumulatorPreparationLimits::new(
                                            8, 20_000, 20_000, 8_192, 200_000,
                                        )
                                        .expect("accumulator limits"),
                                        &resolver,
                                        custody,
                                    )
                                    .expect("staged outcome cancellation checkpoint");
                                complete_prepared_checkpoint(
                                    prepared_checkpoint,
                                    prior,
                                    checkpoint_input("recovery-cancellation"),
                                    UnixTimeV1::new(70).expect("cancellation checkpoint time"),
                                    &checkpoint_fields.schema_sha256,
                                    &checkpoint_signing,
                                    custody,
                                    anchor,
                                    |_, _, _| Ok(signature()),
                                )
                                .expect("staged outcome cancellation custody");
                            }
                            let evidence = crate::resident::operation_custody::read_operation_start_for_request(
                                custody,
                                &selected_store,
                                request,
                                1 << 20,
                            )
                            .expect("operation start read")
                            .expect("operation start evidence");
                            let mut admit_request = |reference: &pale_core::records::ResidentRecordRefV1,
                                                     wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                                let expected = if reference
                                    == operation.request_coordinate().record_ref()
                                {
                                    request
                                } else {
                                    assert_eq!(reference, &cancellation_request_ref);
                                    cancellation_request
                                        .as_ref()
                                        .expect("retained cancellation request")
                                };
                                assert_eq!(
                                    pale_core::canon::to_canonical_string(&wire)
                                        .expect("request wire encoding"),
                                    pale_core::canon::to_canonical_string(expected)
                                        .expect("expected request wire encoding")
                                );
                                Ok(expected.clone())
                            };
                            let mut admit_handle_authority =
                                |_: &pale_core::records::OperationHandlePreimageV1,
                                 parent: Option<&pale_core::records::OperationHandleV1>| {
                                    assert!(parent.is_none());
                                    Ok(())
                                };
                            let mut verify_handle_signature = |_: &[u8],
                                                               _: &pale_core::records::OperationHandlePreimageV1,
                                                               _: &pale_core::records::OperationHandleDigest,
                                                               _: &pale_core::records::CanonicalSignatureV1| {
                                Ok(())
                            };
                            let mut admit_checkpoint =
                                |_: &pale_core::records::ResidentRecordRefV1,
                                 wire: pale_core::records::ControlStateCheckpointWireV1| {
                                    pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                        wire,
                                        &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                        &checkpoint_signing,
                                        |_, _, _, _| Ok(()),
                                    )
                                };
                            let mut resolve_observation_signer =
                                |_: &pale_core::records::ResidentRecordRefV1,
                                 _: &pale_core::records::OperationObservationWireV1| {
                                    Ok(parents.signing.clone())
                                };
                            let mut admit_observation_state =
                                |_: &OperationObservationStateV1| Ok(());
                            let mut admit_rotation = |_: &pale_core::records::ResidentRecordRefV1,
                                                      _: &pale_core::records::ArtifactDigest,
                                                      _: &pale_core::records::ResidentRecordRefV1,
                                                      _: &pale_core::records::ArtifactDigest| {
                                Ok(())
                            };
                            let mut verify_observation_signature =
                                |_: &[u8],
                                 _: &pale_core::records::OperationObservationPreimageV1,
                                 _: &pale_core::records::OperationObservationDigest,
                                 _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
                            let mut verify_checkpoint_signature =
                                |_: &[u8],
                                 _: &pale_core::records::ControlStateCheckpointPreimageV1,
                                 _: &pale_core::records::ControlStateCheckpointDigest,
                                 _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
                            let restart = native_recovery_input(recovery());
                            let disposition = host
                                .classify_existing_operation(
                                    ResidentProcessExistingOperationStart {
                                        selector: selector.clone(),
                                        evidence: Box::new(evidence),
                                    },
                                    native_recovery_association(parents.signing, &restart),
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &sessions,
                                        anchor,
                                    },
                                    restart,
                                    OperationControlRecoveryAdmission {
                                        admit_request: &mut admit_request,
                                        admit_handle_authority: &mut admit_handle_authority,
                                        verify_handle_signature: &mut verify_handle_signature,
                                        admit_checkpoint: &mut admit_checkpoint,
                                        resolve_observation_signer: &mut resolve_observation_signer,
                                        admit_observation_state: &mut admit_observation_state,
                                        admit_rotation: &mut admit_rotation,
                                        verify_observation_signature:
                                            &mut verify_observation_signature,
                                        verify_checkpoint_signature:
                                            &mut verify_checkpoint_signature,
                                    },
                                )
                                .expect("staged native classification");
                            let attempt = match disposition {
                                ResidentProcessOperationRestartDisposition::Native(attempt) => {
                                    attempt
                                }
                                _ => panic!("staged native outcome must retain its owner"),
                            };
                            macro_rules! recover_staged {
                                ($clock:expr, $signer:expr, $prefix:expr, $terminal_ref:expr, $terminal_checkpoint:expr) => {
                                    host.recover_staged_native(
                                        ResidentDeploymentOperationAccess {
                                            store: &selected_store,
                                            custody,
                                            sessions: &sessions,
                                            anchor,
                                        },
                                        ResidentProcessStagedNativeRecoveryInput {
                                            attempt: &attempt,
                                            current: parents.current,
                                            registry: &registry,
                                            contract_set: &contracts,
                                            maximum_selection_bytes: 1 << 20,
                                            maximum_record_bytes: 1 << 20,
                                            maximum_outcome_bytes: 1 << 20,
                                            recording:
                                                ResidentProcessStagedNativeRecoveryRecordingInput {
                                                    result_ref: record_ref(&format!(
                                                        "process-native/{}/result",
                                                        $prefix
                                                    )),
                                                    diagnostic_ref: record_ref(&format!(
                                                        "process-native/{}/diagnostic",
                                                        $prefix
                                                    )),
                                                    terminal_ref: $terminal_ref,
                                                    terminal_checkpoint: $terminal_checkpoint,
                                                    clock: $clock,
                                                    resolve_store: &resolver,
                                                    checkpoint_schema: &checkpoint_fields
                                                        .schema_sha256,
                                                    checkpoint_authority: &checkpoint_signing,
                                                    signing: SignedResidentAdmissionContextV1 {
                                                        expected_schema_sha256: &observation_schema,
                                                        signing_authority: $signer,
                                                    },
                                                    maximum_diagnostic_bytes: 65_536,
                                                    accumulator_limits:
                                                        AccumulatorPreparationLimits::new(
                                                            8, 20_000, 20_000, 8_192, 200_000,
                                                        )
                                                        .expect("accumulator limits"),
                                                },
                                        },
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| Ok(signature()),
                                        |_, _, _| Ok(signature()),
                                        |_, _, _| Ok(signature()),
                                    )
                                };
                            }

                            let foreign =
                                owner_native_signing_authority(selector.principal_id.clone());
                            let foreign_clock = IncreasingClock::starting_at(70, None);
                            let foreign_result = recover_staged!(
                                &foreign_clock,
                                &foreign,
                                "foreign-recovered-terminal",
                                record_ref("process-native/foreign-recovered-terminal"),
                                checkpoint_input("foreign-recovered-terminal")
                            );
                            assert!(matches!(
                                foreign_result,
                                Err(ResidentProcessStagedNativeRecoveryFailure::Selection(
                                    ResidentProcessHostSelectionError::NativeSigningAuthority
                                ))
                            ));
                            assert_eq!(foreign_clock.reads(), 0);

                            let alias_clock = IncreasingClock::starting_at(70, None);
                            let alias_result = recover_staged!(
                                &alias_clock,
                                parents.signing,
                                "aliased-recovered-terminal",
                                attempt.start.control.observation().selected().0.clone(),
                                checkpoint_input("aliased-recovered-terminal")
                            );
                            assert!(matches!(
                                alias_result,
                                Err(ResidentProcessStagedNativeRecoveryFailure::Admission(
                                    crate::resident::operation_dispatch::OperationAttemptError::Association
                                ))
                            ));
                            assert_eq!(alias_clock.reads(), 0);

                            if cancellation_request.is_some() {
                                let request_alias_clock = IncreasingClock::starting_at(70, None);
                                let request_alias_result = recover_staged!(
                                    &request_alias_clock,
                                    parents.signing,
                                    "cancellation-request-alias",
                                    cancellation_request_ref.clone(),
                                    checkpoint_input("cancellation-request-alias")
                                );
                                assert!(matches!(
                                    request_alias_result,
                                    Err(ResidentProcessStagedNativeRecoveryFailure::Admission(
                                        crate::resident::operation_dispatch::OperationAttemptError::Association
                                    ))
                                ));
                                assert_eq!(request_alias_clock.reads(), 0);

                                let checkpoint_alias_clock = IncreasingClock::starting_at(70, None);
                                let selected_checkpoint_id = attempt
                                    .start
                                    .control
                                    .observation()
                                    .selected_checkpoint_record()
                                    .1
                                    .control_checkpoint_id()
                                    .clone();
                                let checkpoint_alias_result = recover_staged!(
                                    &checkpoint_alias_clock,
                                    parents.signing,
                                    "staged-checkpoint-alias",
                                    record_ref("process-native/staged-checkpoint-alias"),
                                    OperationCheckpointInput {
                                        reference: record_ref(
                                            "process-native/staged-checkpoint-alias-checkpoint"
                                        ),
                                        id: selected_checkpoint_id,
                                    }
                                );
                                assert!(matches!(
                                    checkpoint_alias_result,
                                    Err(ResidentProcessStagedNativeRecoveryFailure::Admission(
                                        crate::resident::operation_dispatch::OperationAttemptError::Association
                                    ))
                                ));
                                assert_eq!(checkpoint_alias_clock.reads(), 0);
                            }

                            let elapsed_clock =
                                IncreasingClock::starting_at(handle.retain_until().get() + 1, None);
                            let elapsed_result = recover_staged!(
                                &elapsed_clock,
                                parents.signing,
                                "elapsed-recovered-terminal",
                                record_ref("process-native/elapsed-recovered-terminal"),
                                checkpoint_input("elapsed-recovered-terminal")
                            );
                            let Err(ResidentProcessStagedNativeRecoveryFailure::Recording(
                                NativeOperationRecordingFailure::TerminalPreparation {
                                    failure,
                                    ..
                                },
                            )) = elapsed_result
                            else {
                                panic!("elapsed recovery time must retain terminal preparation")
                            };
                            assert!(matches!(
                                *failure,
                                OperationAppendFailure::Operation(
                                    crate::resident::operation_dispatch::OperationAttemptError::RetentionElapsed
                                )
                            ));
                            assert_eq!(elapsed_clock.reads(), 1);

                            let recovered_clock = IncreasingClock::starting_at(70, None);
                            let completed = recover_staged!(
                                &recovered_clock,
                                parents.signing,
                                "recovered-terminal",
                                record_ref("process-native/recovered-terminal"),
                                checkpoint_input("recovered-terminal")
                            )
                            .expect("staged native terminal recovery");
                            assert_eq!(recovered_clock.reads(), 1);
                            assert_eq!(
                                completed.result().is_ok(),
                                matches!(
                                    case,
                                    NativeProcessCase::StagedRecoverySuccess
                                        | NativeProcessCase::StagedRecoverySuccessAfterCancellation
                                )
                            );
                            assert_eq!(
                                completed.observation().1.predecessor_ref().as_ref(),
                                Some(attempt.start.control.observation().selected_head().0)
                            );
                            if let Ok(result) = completed.result() {
                                assert_eq!(result.completed_at(), &UnixTimeV1::new(66).unwrap());
                            }
                        }
                        NativeProcessCase::TerminalCustodyContinuation => {
                            let Err(NativeOperationInvokeFailure::Recording(
                                NativeOperationRecordingFailure::TerminalCustody { failure },
                            )) = result
                            else {
                                panic!("expected retained terminal custody")
                            };
                            let OperationAppendFailure::Commit { failure, .. } = *failure else {
                                panic!("expected terminal commit uncertainty")
                            };
                            let pending = match failure.reconcile_exact(custody) {
                                Ok(pending) => pending,
                                Err(unavailable) => unavailable
                                    .reconcile_exact(custody)
                                    .expect("exact terminal successor after unavailable read-back"),
                            };
                            let completed = pending
                                .advance_anchor(anchor)
                                .expect("terminal successor anchor");
                            assert!(completed
                                .candidate()
                                .record(
                                    pale_ports::OperationCustodyRecordKind::OperationObservation
                                )
                                .is_some());
                        }
                        NativeProcessCase::InvocationBoundary => {
                            assert!(matches!(
                                result,
                                Err(NativeOperationInvokeFailure::Invocation(_))
                            ));
                            assert_eq!(clock.reads(), 3);
                            assert_eq!(signs.get(), 2);
                        }
                        NativeProcessCase::CurrentAuthorityRejection => unreachable!(),
                        NativeProcessCase::AssociationRejection => unreachable!(),
                        NativeProcessCase::BadStartEvidence => unreachable!(),
                        NativeProcessCase::CancellationBeforeDispatch => unreachable!(),
                    }
                },
            );
        },
    );
}

fn open_provider_consult_child<'input, 'operation, 'call, Authority, Executor>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderOpenTurn<'input, 'operation, 'call, Authority, Executor>,
) -> ResidentOperationTaskKey
where
    'operation: 'call,
    'operation: 'input,
    'call: 'input,
    Authority: AmbientSessionAuthorityPort,
    Executor: ResidentOperationTaskExecutor<
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildOutput,
    >,
{
    let opened = host
        .open_consultation_provider_child(selector, turn)
        .unwrap_or_else(|_| panic!("exact provider Consult turn is selected"))
        .unwrap_or_else(|error| panic!("provider child recovery failed: {error:?}"));
    match opened {
        ResidentConsultationTurnDisposition::Progress(
            ResidentConsultationTurnProgress::Offered { dispatch, .. },
        ) => dispatch,
        ResidentConsultationTurnDisposition::NeedsRecovery(recovery) => match *recovery {
            ResidentConsultationTurnNeedsRecovery::Offer(offer) => {
                let (_, _, failure) = offer.into_parts();
                match *failure {
                    ResidentConsultationTaskOwnerAcceptanceFailure::Acceptance(failure) => {
                        match *failure {
                            ResidentConsultationTaskAcceptanceFailure::RetentionUnopened {
                                reason:
                                    ResidentProcessConsultProviderChildRetentionFailure::Unopened(
                                        reason,
                                    ),
                                ..
                            } => match reason {
                                ResidentProcessConsultProviderUnopenedFailure::Selection(
                                    reason,
                                ) => {
                                    panic!("provider child selection failed: {reason:?}")
                                }
                                ResidentProcessConsultProviderUnopenedFailure::Admission(
                                    reason,
                                ) => {
                                    panic!("provider child admission failed: {reason:?}")
                                }
                            },
                            failure => panic!("provider child acceptance failed: {failure:?}"),
                        }
                    }
                    failure => panic!("provider child offer failed: {failure:?}"),
                }
            }
            _ => panic!("provider child required non-offer recovery"),
        },
        _ => panic!("provider child did not reach its task owner"),
    }
}

fn poll_provider_consult_child<'input, 'operation, 'call, Executor>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderPollTurn<'input, 'operation, Executor>,
) -> Box<
    ResidentConsultationCompletionReady<
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildOutput,
    >,
>
where
    'operation: 'call,
    Executor: ResidentOperationTaskExecutor<
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildOutput,
    >,
{
    let polled = host
        .poll_consultation_provider_child(selector, turn)
        .unwrap_or_else(|_| panic!("exact provider completion is selected"));
    let ResidentConsultationTurnDisposition::Progress(
        ResidentConsultationTurnProgress::CompletionReady(ready),
    ) = polled
    else {
        panic!("provider completion remains task-owned")
    };
    ready
}

fn finalize_provider_consult_child<'input, 'operation, 'call>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderFinalizeTurn<'input, 'operation, 'call>,
) -> ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>
where
    'operation: 'call,
{
    host.finalize_consultation_provider_child(selector, turn)
        .unwrap_or_else(|_| panic!("provider failure reaches conclusive custody"))
}

fn finalize_provider_consult_child_continuation<'input, 'operation, 'call>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderFinalizeTurn<'input, 'operation, 'call>,
) -> ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>
where
    'operation: 'call,
{
    match host.finalize_consultation_provider_child(selector, turn) {
        Err(failure) => match *failure {
            ResidentProcessConsultProviderFinalizeFailure::Continuation(continuation) => {
                *continuation
            }
            ResidentProcessConsultProviderFinalizeFailure::BeforeContinuation(_) => {
                panic!("provider completion stopped before its expected continuation")
            }
        },
        Ok(_) => panic!("provider completion unexpectedly skipped its injected refusal"),
    }
}

fn accepting_provider_recording_attempt(
    observation_signatures: Rc<Cell<usize>>,
) -> ResidentProcessConsultProviderRecordingAttempt<'static> {
    ResidentProcessConsultProviderRecordingAttempt {
        reconcile: Box::new(|_| Ok(())),
        rotate: Box::new(|_, _, _, _| Ok(())),
        sign_observation: Box::new(move |_, _, _| {
            observation_signatures.set(observation_signatures.get() + 1);
            Ok(signature())
        }),
        sign_checkpoint: Box::new(|_, _, _| Ok(signature())),
    }
}

fn rejecting_provider_recording_attempt(
    observation_signatures: Rc<Cell<usize>>,
) -> ResidentProcessConsultProviderRecordingAttempt<'static> {
    ResidentProcessConsultProviderRecordingAttempt {
        reconcile: Box::new(|_| Ok(())),
        rotate: Box::new(|_, _, _, _| Ok(())),
        sign_observation: Box::new(move |_, _, _| {
            observation_signatures.set(observation_signatures.get() + 1);
            Err(modeled_signing_failure())
        }),
        sign_checkpoint: Box::new(|_, _, _| Ok(signature())),
    }
}

fn rejecting_provider_checkpoint_attempt(
    checkpoint_signatures: Rc<Cell<usize>>,
) -> ResidentProcessConsultProviderRecordingAttempt<'static> {
    ResidentProcessConsultProviderRecordingAttempt {
        reconcile: Box::new(|_| Ok(())),
        rotate: Box::new(|_, _, _, _| Ok(())),
        sign_observation: Box::new(|_, _, _| Ok(signature())),
        sign_checkpoint: Box::new(move |_, _, _| {
            checkpoint_signatures.set(checkpoint_signatures.get() + 1);
            Err(modeled_signing_failure())
        }),
    }
}

fn assert_provider_completion_phase(
    continuation: &ResidentProcessConsultProviderCompletionContinuation<'_, '_, '_>,
    expected: ProviderConsultChildPath,
) {
    let matches = matches!(
        (expected, continuation),
        (
            ProviderConsultChildPath::CompletionRecording,
            ResidentProcessConsultProviderCompletionContinuation::Recording { .. }
        ) | (
            ProviderConsultChildPath::CompletionLostRecording,
            ResidentProcessConsultProviderCompletionContinuation::LostRecording { .. }
        ) | (
            ProviderConsultChildPath::CompletionClassification,
            ResidentProcessConsultProviderCompletionContinuation::Classification { .. }
        ) | (
            ProviderConsultChildPath::CompletionSettlement,
            ResidentProcessConsultProviderCompletionContinuation::Settlement { .. }
        )
    );
    assert!(matches, "provider completion retained the expected phase");
}

fn provider_completion_recording_turn<'input, 'operation, 'call>(
    continuation: ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>,
    recording: ResidentProcessConsultProviderRecordingAttempt<'input>,
) -> ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call> {
    continuation
        .with_recording_attempt(recording)
        .unwrap_or_else(|_| panic!("recording phase requires one fresh callback bundle"))
}

fn provider_completion_checkpoint_turn<'input, 'operation, 'call>(
    continuation: ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>,
    checkpoint_signatures: Rc<Cell<usize>>,
) -> ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call> {
    continuation
        .with_checkpoint_signer(Box::new(move |_, _, _| {
            checkpoint_signatures.set(checkpoint_signatures.get() + 1);
            Ok(signature())
        }))
        .unwrap_or_else(|_| panic!("prepared phase requires only one checkpoint signer"))
}

fn provider_completion_callback_free_turn<'input, 'operation, 'call>(
    continuation: ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>,
) -> ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call> {
    continuation
        .without_recording_attempt()
        .unwrap_or_else(|_| panic!("this completion phase requires no recording callbacks"))
}

fn assert_provider_completion_requires_fresh_recording<'input, 'operation, 'call>(
    continuation: ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>,
) -> ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call> {
    match continuation.without_recording_attempt() {
        Err(continuation) => *continuation,
        Ok(_) => panic!("selected recording refusal must not return consumed callbacks"),
    }
}

fn preserve_provider_completion_on_wrong_selector<'input, 'operation, 'call>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    mut turn: ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>,
) -> ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>
where
    'operation: 'call,
{
    let mut wrong = selector.clone();
    wrong.principal_id = pale_core::records::ResidentPrincipalIdV1::parse(
        "process-consult-provider-wrong-principal",
    )
    .expect("wrong provider continuation principal");
    for _ in 0..2 {
        let Err(ResidentProcessConsultProviderContinuationFailure::Selection(refusal)) =
            host.continue_consultation_provider_completion(&wrong, turn)
        else {
            panic!("wrong selector returns the complete provider continuation")
        };
        let (returned, _) = (*refusal).into_parts();
        turn = returned;
    }
    turn
}

fn continue_provider_completion_expect_retry<'input, 'operation, 'call>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>,
) -> ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>
where
    'operation: 'call,
{
    match host.continue_consultation_provider_completion(selector, turn) {
        Err(ResidentProcessConsultProviderContinuationFailure::Completion(continuation)) => {
            *continuation
        }
        Err(ResidentProcessConsultProviderContinuationFailure::Selection(_)) => {
            panic!("exact selector reached the provider completion phase")
        }
        Ok(_) => panic!("provider completion unexpectedly skipped its repeated refusal"),
    }
}

fn join_provider_consult_child<'input, 'operation, 'call, Authority>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    turn: ResidentProcessConsultProviderJoinTurn<'input, 'operation, 'call, Authority>,
) -> Box<
    CompletedResidentConsultationTerminal<
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildOutput,
    >,
>
where
    'operation: 'call,
    'operation: 'input,
    'call: 'input,
    Authority: AmbientSessionAuthorityPort,
{
    let ResidentProcessConsultProviderJoinTurn {
        association,
        access,
        authority,
        completion,
        input,
        checkpoint,
        resolve_store,
        admit_evidence,
        admit_collection,
        rotate,
        sign_observation,
        sign_checkpoint,
    } = turn;
    let joined = host.complete_consultation_provider_parent(
        association,
        access,
        authority,
        completion,
        input,
        checkpoint,
        resolve_store,
        admit_evidence,
        admit_collection,
        rotate,
        sign_observation,
        sign_checkpoint,
    );
    match joined {
        Ok(ResidentProcessConsultProviderParentDisposition::Joined(completed)) => completed,
        Ok(ResidentProcessConsultProviderParentDisposition::Nonterminal(_)) => {
            panic!("conclusive provider failure became nonterminal")
        }
        Err(failure) => match *failure {
            ResidentProcessConsultationTerminalJoinFailure::Selection { reason, .. } => {
                panic!("provider join selection failed: {reason:?}")
            }
            ResidentProcessConsultationTerminalJoinFailure::Preflight { reason, .. } => {
                panic!("provider join preflight failed: {reason:?}")
            }
            ResidentProcessConsultationTerminalJoinFailure::Admission { reason, .. } => {
                panic!("provider join admission failed: {reason:?}")
            }
            ResidentProcessConsultationTerminalJoinFailure::Custody(reason) => {
                panic!("provider join custody remained incomplete: {reason:?}")
            }
            ResidentProcessConsultationTerminalJoinFailure::CustodyPreflight { reason, .. } => {
                panic!("provider join custody preflight failed: {reason:?}")
            }
            ResidentProcessConsultationTerminalJoinFailure::CustodySelection { reason, .. } => {
                panic!("provider join custody selection failed: {reason:?}")
            }
            ResidentProcessConsultationTerminalJoinFailure::Release { reason, .. } => {
                panic!("provider join release failed: {reason:?}")
            }
            ResidentProcessConsultationTerminalJoinFailure::ReleasePreflight { reason, .. } => {
                panic!("provider join release preflight failed: {reason:?}")
            }
            ResidentProcessConsultationTerminalJoinFailure::ReleaseSelection { reason, .. } => {
                panic!("provider join release selection failed: {reason:?}")
            }
        },
    }
}

#[derive(Clone, Copy)]
struct ConsultChildFixture<'a> {
    label: &'a str,
    child: &'a crate::resident::consultation::tests::IssuedTestChild,
    parent: &'a crate::resident::IssuedLocalOperation,
    generation: &'a AdmittedCompiledGeneration,
    parents: RecoveryParents<'a>,
}

#[derive(Clone, Copy)]
struct RecoveredConsultChild<'a> {
    parent_start: &'a ResidentProcessRecoveredOperationStart,
    current: &'a RecoveredConsultationCollection,
    batch: &'a ConsultationChildDispatchBatch,
    candidate: &'a ConsultationChildDispatchCandidate,
    recovered: &'a ResidentProcessRecoveredOperationResolution,
    observed_at: UnixTimeV1,
}

type ProviderConsultChildTail =
    for<'fixture, 'recovered, 'host, 'application, 'custody, 'anchor> fn(
        ConsultChildFixture<'fixture>,
        RecoveredConsultChild<'recovered>,
        &'host mut ResidentProcessHost<'application, UnprovedHeldWriter>,
        &'custody mut dyn pale_ports::OperationCustodyPort,
        &'anchor mut dyn pale_ports::AntiRollbackAnchorPort,
    );

#[derive(Clone, Copy, Eq, PartialEq)]
enum ProviderConsultChildPath {
    Fresh,
    BoundedUnknown,
    SameProcessResume,
    RecoveredReferenceCollision,
    CompletionRecording,
    CompletionLostRecording,
    CompletionClassification,
    CompletionSettlement,
}

impl ProviderConsultChildPath {
    const fn is_resume(self) -> bool {
        matches!(
            self,
            Self::SameProcessResume | Self::RecoveredReferenceCollision
        )
    }

    const fn is_lost_completion(self) -> bool {
        matches!(
            self,
            Self::BoundedUnknown | Self::CompletionLostRecording | Self::CompletionSettlement
        )
    }
}

macro_rules! define_provider_consult_tail {
    ($name:ident, $path:expr) => {
        fn $name(
            fixture: ConsultChildFixture<'_>,
            recovered_child: RecoveredConsultChild<'_>,
            host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
            custody: &mut dyn pale_ports::OperationCustodyPort,
            anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
        ) {
            run_provider_consult_child_tail(fixture, recovered_child, host, custody, anchor, $path);
        }
    };
}

define_provider_consult_tail!(fresh_provider_consult_tail, ProviderConsultChildPath::Fresh);
define_provider_consult_tail!(
    bounded_unknown_provider_consult_tail,
    ProviderConsultChildPath::BoundedUnknown
);
define_provider_consult_tail!(
    same_process_provider_consult_tail,
    ProviderConsultChildPath::SameProcessResume
);
define_provider_consult_tail!(
    colliding_recovered_provider_consult_tail,
    ProviderConsultChildPath::RecoveredReferenceCollision
);
define_provider_consult_tail!(
    recording_provider_consult_tail,
    ProviderConsultChildPath::CompletionRecording
);
define_provider_consult_tail!(
    lost_recording_provider_consult_tail,
    ProviderConsultChildPath::CompletionLostRecording
);
define_provider_consult_tail!(
    classification_provider_consult_tail,
    ProviderConsultChildPath::CompletionClassification
);
define_provider_consult_tail!(
    settlement_provider_consult_tail,
    ProviderConsultChildPath::CompletionSettlement
);

#[inline(never)]
fn run_provider_consult_test_path<R>(run: impl FnOnce() -> R) -> R {
    run()
}

fn run_provider_consult_child_tail(
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    path: ProviderConsultChildPath,
) {
    let ConsultChildFixture {
        label,
        child,
        parent,
        generation,
        parents,
    } = fixture;
    let RecoveredConsultChild {
        parent_start,
        current,
        batch,
        candidate,
        recovered,
        observed_at,
    } = recovered_child;
    let selected_store = child.initial_candidate.store().clone();
    let foreign_store = store(
        "process-consult-provider-foreign-store",
        "process-consult-provider-foreign-incarnation",
    );
    let selector = selector_from_signing_authority(parents.signing);
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let session = parents.session.lease();
    let sessions = RestartSessions(SessionLeaseBody::new(
        selected_store.clone(),
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                .expect("session wire")
                .into_bytes(),
        )
        .expect("bounded session wire"),
    ));
    let mut historical = RecoveryResolver::default();
    historical.insert_generation(parents.current.generation.compiled_generation);
    historical.insert_active(parents.current.generation.active_head);
    historical.insert_fence(parents.current.generation.activation_fence);
    historical.insert_effective_profile(parents.current.generation.effective_profile);
    let child_handle = crate::resident::record_projection::coordinate(
        child.operation.handle().0,
        child
            .operation
            .handle()
            .1
            .canonical_operation_handle_sha256()
            .as_str(),
    )
    .expect("child recovery coordinate");
    let observation_limits =
        OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20).expect("observation bounds");
    let checkpoint_limits =
        OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).expect("checkpoint bounds");
    let membership_limits =
        AccumulatorMembershipLimits::new(8, 20_000, 200_000).expect("membership bounds");
    let checkpoint_profile = crate::local_operation_start_fixture::profile();
    let mut checkpoint_fields = checkpoint_preimage_at_store(
        child.checkpoint.domain_sequence().get() + 1,
        Some((
            child.checkpoint_ref.clone(),
            child
                .checkpoint
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
        &selected_store,
    );
    checkpoint_fields.checkpoint_domain_id = child.checkpoint.checkpoint_domain_id().clone();
    checkpoint_fields.checkpoint_profile_ref = child.checkpoint.checkpoint_profile_ref().clone();
    checkpoint_fields.checkpoint_profile_sha256 =
        child.checkpoint.checkpoint_profile_sha256().clone();
    checkpoint_fields.domain_epoch = *child.checkpoint.domain_epoch();
    let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
    let recovery = || OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: &selected_store,
                profile: &checkpoint_profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority: &checkpoint_signing,
                historical: &historical,
                maximum_record_bytes: 1 << 20,
                maximum_operation_lease_bytes: 1 << 20,
                maximum_session_lease_bytes: 1 << 20,
                membership_limits,
                observation_limits,
                checkpoint_limits,
            },
            expected_handle: &child_handle,
            maximum_parent_depth: 1,
        },
        observation: OperationObservationSelection::SelectedHead,
        observation_limits,
        checkpoint_limits,
        membership_limits,
    };
    let restart = || native_recovery_input(recovery());

    let classify = |driver: &ResidentProcessConsultChildDriver<'_, '_>,
                    custody: &mut dyn pale_ports::OperationCustodyPort,
                    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
                    selected_parent: &ResidentProcessRecoveredOperationStart,
                    selected_current: &RecoveredConsultationCollection,
                    selected_batch: &ConsultationChildDispatchBatch,
                    selected_candidate: &ConsultationChildDispatchCandidate,
                    selected_dispatch: Option<&ImmutableRecordReferenceV1>,
                    maximum_parent_depth: usize,
                    selected_observed_at: UnixTimeV1| {
        let mut admit_request =
            |reference: &pale_core::records::ResidentRecordRefV1,
             wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                let expected = if reference == &child.request_ref {
                    &child.request
                } else if reference == child.parent_request.0.record_ref() {
                    &child.parent_request.1
                } else {
                    let cancellation = super::a2a_cancel_task::cancellation_request(
                        &child.request,
                        child.operation.handle().1,
                    );
                    assert_eq!(
                        pale_core::canon::to_canonical_string(&wire)
                            .expect("cancellation request wire encoding"),
                        pale_core::canon::to_canonical_string(&cancellation)
                            .expect("expected cancellation request encoding")
                    );
                    return Ok(cancellation);
                };
                assert_eq!(
                    pale_core::canon::to_canonical_string(&wire).expect("request wire encoding"),
                    pale_core::canon::to_canonical_string(expected)
                        .expect("expected request encoding")
                );
                Ok(expected.clone())
            };
        let mut admit_handle_authority =
            |preimage: &pale_core::records::OperationHandlePreimageV1,
             recovered_parent: Option<&pale_core::records::OperationHandleV1>| {
                if &preimage.operation_handle_id == child.operation.handle().1.operation_handle_id()
                {
                    let recovered_parent =
                        recovered_parent.expect("child recovery retains its Consult parent");
                    assert_eq!(
                        recovered_parent.canonical_operation_handle_sha256(),
                        parent.handle().1.canonical_operation_handle_sha256()
                    );
                } else {
                    assert_eq!(
                        &preimage.operation_handle_id,
                        parent.handle().1.operation_handle_id()
                    );
                    assert!(recovered_parent.is_none());
                }
                Ok(())
            };
        let mut verify_handle_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
        let mut admit_checkpoint = |_: &_, wire| {
            pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                wire,
                &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                &checkpoint_signing,
                |_, _, _, _| Ok(()),
            )
        };
        let mut resolve_observation_signer = |_: &_, _: &_| Ok(parents.signing.clone());
        let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
        let mut verify_observation_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
        let mut verify_checkpoint_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
        let mut selected_restart = restart();
        selected_restart.maximum_parent_depth = maximum_parent_depth;
        driver.classify_child_operation(
            native_recovery_association(parents.signing, &selected_restart),
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions: &sessions,
                anchor,
            },
            selected_restart,
            &mut OperationControlRecoveryAdmission {
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
            selected_parent,
            selected_current,
            selected_batch,
            selected_candidate,
            selected_dispatch,
            selected_observed_at,
        )
    };
    let initial_record = child
        .initial_candidate
        .record(OperationCustodyRecordKind::OperationObservation)
        .expect("child initial observation");
    with_provider_selection_for_operation(
        &child.operation,
        &child.request_ref,
        &child.request,
        generation,
        &parents,
        "attempt-1",
        record_ref("process-consult-provider/selection"),
        |provider_request, provider_admission, attempt, selection_record| {
            let dispatch_floor = parents.current.now.max(*child.checkpoint.issued_at()).get();
            let frontier = ProviderInvocationEvidenceFrontier::new(
                ProviderInvocationEvidenceSubject::from_request(provider_request),
                record_ref("provider-evidence"),
                artifact_digest('5'),
                UnixTimeV1::new(dispatch_floor + 4).expect("provider evidence time"),
                UnixTimeV1::new(dispatch_floor + 14).expect("provider evidence expiry"),
            )
            .expect("provider evidence frontier");
            let coordinate =
                ProviderInvocationObservationCoordinate::capture(provider_request, frontier)
                    .expect("provider observation coordinate");
            let provider = ResultPort::failure(safe_failure_at(
                &child.request,
                parents.signing,
                attempt,
                coordinate,
                UnixTimeV1::new(dispatch_floor + 6).expect("provider start time"),
                UnixTimeV1::new(dispatch_floor + 9).expect("provider completion time"),
            ));
            with_provider_dispatch_inputs(
                label,
                &selected_store,
                provider_request,
                &child.operation,
                &child.request,
                generation,
                provider_admission,
                &parents,
                (&child.checkpoint_ref, &child.checkpoint),
                initial_record,
                &selection_record,
                |dispatch, mut invocation, lost_invocation, provider_clock, provider_store| {
                    let task_limits = ResidentConsultationTaskLimits::new(
                        NonZeroUsize::new(1).expect("one child in flight"),
                        NonZeroUsize::new(1).expect("one roster member"),
                    );
                    if path.is_resume() {
                        run_provider_consult_test_path(|| {
                            let source = {
                                let driver = host
                                    .consultation_child_driver(&selector)
                                    .expect("provider Consult preparation driver");
                                let source = classify(
                                    &driver,
                                    custody,
                                    anchor,
                                    parent_start,
                                    current,
                                    batch,
                                    candidate,
                                    None,
                                    1,
                                    observed_at,
                                )
                                .expect("unopened provider child classifies before retention");
                                drop(driver);
                                source
                            };
                            let ResidentProcessOperationRestartDisposition::OperationResolution(
                                source,
                            ) = source
                            else {
                                panic!("unopened provider child remains at Invoke resolution")
                            };
                            let mut retention_authority = ProvingAuthority;
                            let retained = retain_provider_operation_dispatch(
                                dispatch,
                                &mut retention_authority,
                                custody,
                                anchor,
                                |_, _, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                                |_, _, _| Ok(signature()),
                            )
                            .expect("provider dispatch retained without a provider call");
                            let dispatch_recovery = retained.recovery_fixture();
                            let release = retained.into_recovered_release_fixture();
                            let expected_release_store = release.store().clone();
                            let expected_release_ref = release.observation().0.clone();
                            let expected_release_digest = release
                                .observation()
                                .1
                                .canonical_operation_observation_sha256()
                                .clone();
                            let expected_release_checkpoint = release.checkpoint().0.clone();
                            if path == ProviderConsultChildPath::RecoveredReferenceCollision {
                                invocation.result_ref = release.operation().handle().0.clone();
                                invocation.diagnostic_ref = invocation.result_ref.clone();
                                invocation.reconciliation.record_ref =
                                    release.selection_ref().clone();
                                invocation.terminal_ref = release.observation().0.clone();
                                invocation.terminal_checkpoint.reference =
                                    release.preparation().0.clone();
                                invocation.terminal_checkpoint.id =
                                    release.checkpoint().1.control_checkpoint_id().clone();
                            }
                            let ResidentProcessRecoveredOperationResolution {
                                start:
                                    ResidentProcessRecoveredOperationStart {
                                        selector: source_selector,
                                        control: source_control,
                                        generation_digest,
                                        lineage,
                                    },
                                ..
                            } = *source;
                            let (recovered_operation, _) = source_control.into_parts();
                            let prepared = ResidentProcessRecoveredPreparedAttempt {
                                start: ResidentProcessRecoveredOperationStart {
                                    selector: source_selector,
                                    control: RecoveredOperationControl::fixture(
                                        recovered_operation,
                                        dispatch_recovery
                                            .prepared_recovered(dispatch_recovery.checkpoint()),
                                    ),
                                    generation_digest,
                                    lineage,
                                },
                                selection: dispatch_recovery.selection(),
                            };

                            let mut admit_request =
                            |reference: &pale_core::records::ResidentRecordRefV1,
                             wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                                let expected = if reference == &child.request_ref {
                                    &child.request
                                } else if reference == child.parent_request.0.record_ref() {
                                    &child.parent_request.1
                                } else {
                                    panic!("unexpected resumed provider Consult request")
                                };
                                assert_eq!(
                                    pale_core::canon::to_canonical_string(&wire)
                                        .expect("request wire encoding"),
                                    pale_core::canon::to_canonical_string(expected)
                                        .expect("expected request encoding")
                                );
                                Ok(expected.clone())
                            };
                            let mut admit_handle_authority =
                                |preimage: &pale_core::records::OperationHandlePreimageV1,
                                 recovered_parent: Option<
                                    &pale_core::records::OperationHandleV1,
                                >| {
                                    if &preimage.operation_handle_id
                                        == child.operation.handle().1.operation_handle_id()
                                    {
                                        assert_eq!(
                                            recovered_parent
                                                .expect("resumed provider child retains parent")
                                                .canonical_operation_handle_sha256(),
                                            parent.handle().1.canonical_operation_handle_sha256()
                                        );
                                    } else {
                                        assert_eq!(
                                            &preimage.operation_handle_id,
                                            parent.handle().1.operation_handle_id()
                                        );
                                        assert!(recovered_parent.is_none());
                                    }
                                    Ok(())
                                };
                            let mut verify_handle_signature =
                                |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                            let mut admit_checkpoint = |_: &_, wire| {
                                pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                    &checkpoint_signing,
                                    |_, _, _, _| Ok(()),
                                )
                            };
                            let mut resolve_observation_signer =
                                |_: &_, _: &_| Ok(parents.signing.clone());
                            let mut admit_observation_state =
                                |_: &OperationObservationStateV1| Ok(());
                            let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
                            let mut verify_observation_signature =
                                |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                            let mut verify_checkpoint_signature =
                                |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                            let mut recovery_admission = OperationControlRecoveryAdmission {
                                admit_request: &mut admit_request,
                                admit_handle_authority: &mut admit_handle_authority,
                                verify_handle_signature: &mut verify_handle_signature,
                                admit_checkpoint: &mut admit_checkpoint,
                                resolve_observation_signer: &mut resolve_observation_signer,
                                admit_observation_state: &mut admit_observation_state,
                                admit_rotation: &mut admit_rotation,
                                verify_observation_signature: &mut verify_observation_signature,
                                verify_checkpoint_signature: &mut verify_checkpoint_signature,
                            };
                            let resume_restart = restart();
                            let mut resume_authority = ProvingAuthority;
                            let mut executor = DeferredConsultChildExecutor::default();
                            executor.refuse_next();
                            let turn = ResidentProcessConsultProviderResumeTurn {
                                association: native_recovery_association(
                                    parents.signing,
                                    &resume_restart,
                                ),
                                access: ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                authority: &mut resume_authority,
                                task_limits,
                                observed_at,
                                current: current.clone(),
                                planned: batch,
                                candidate,
                                parent: parent_start,
                                phase: ResidentProcessConsultProviderResumePhase::DispatchPrepared(
                                    &prepared,
                                ),
                                release,
                                invocation,
                                provider_admission,
                                restart: Box::new(|| native_recovery_input(recovery())),
                                admission: &mut recovery_admission,
                                executor: &mut executor,
                            };
                            let reads_before_refusal = provider_clock.reads();
                            let mut foreign_selector = selector.clone();
                            foreign_selector.runtime_authority_ref =
                                record_ref("process-consult-provider-resume/foreign-runtime");
                            let refused = match host
                                .resume_consultation_provider_child(&foreign_selector, turn)
                            {
                                Err(refused) => refused,
                                Ok(_) => {
                                    panic!("foreign selector must return the whole resume turn")
                                }
                            };
                            assert_eq!(provider_clock.reads(), reads_before_refusal);
                            assert_eq!(provider.calls(), 0);
                            let (turn, reason) = refused.into_parts();
                            assert_eq!(turn.executor.acceptance_attempts(), 0);
                            assert!(matches!(
                                reason,
                                ResidentProcessHostSelectionError::ConsultationChildAssociation
                            ));
                            if path == ProviderConsultChildPath::RecoveredReferenceCollision {
                                let reads_before_collision = provider_clock.reads();
                                let resolves_before_collision = provider_store.calls();
                                let resumed = host
                                    .resume_consultation_provider_child(&selector, turn)
                                    .unwrap_or_else(|_| {
                                        panic!("exact selector reaches collision validation")
                                    });
                                let ResidentProcessConsultProviderResumeFailure::Selection {
                                    release,
                                    reason,
                                } = resumed.expect_err(
                                    "reused recovered-call references must refuse before I/O",
                                )
                                else {
                                    panic!("reference collision must return the exact release")
                                };
                                assert!(matches!(
                                    reason,
                                    ResidentProcessHostSelectionError::ProviderOperationAuthority
                                ));
                                assert_eq!(release.store(), &expected_release_store);
                                assert_eq!(release.observation().0, &expected_release_ref);
                                assert_eq!(
                                    release
                                        .observation()
                                        .1
                                        .canonical_operation_observation_sha256(),
                                    &expected_release_digest
                                );
                                assert_eq!(release.checkpoint().0, &expected_release_checkpoint);
                                assert_eq!(provider_clock.reads(), reads_before_collision);
                                assert_eq!(provider_store.calls(), resolves_before_collision);
                                assert_eq!(executor.acceptance_attempts(), 0);
                                assert_eq!(provider.calls(), 0);
                                return;
                            }
                            let resumed =
                                match host.resume_consultation_provider_child(&selector, turn) {
                                    Ok(resumed) => resumed,
                                    Err(_) => panic!("exact selector must reach the resumed owner"),
                                };
                            let failure = match resumed {
                                Err(failure) => failure,
                                Ok(_) => panic!("first resumed executor offer must be refused"),
                            };
                            let retry = match failure {
                                ResidentProcessConsultProviderResumeFailure::Acceptance(
                                    failure,
                                ) => match *failure {
                                    ResidentConsultationPreparedResumeFailure::Refused(refusal) => {
                                        match *refusal {
                                            ResidentConsultationTaskRefusal::Executor(retry) => {
                                                retry
                                            }
                                            other => panic!(
                                                "resumed work returned through wrong refusal: {other:?}"
                                            ),
                                        }
                                    }
                                    other => {
                                        panic!("prepared resumed work was not returned: {other:?}")
                                    }
                                },
                                other => panic!("resume stopped before task acceptance: {other:?}"),
                            };
                            assert_eq!(provider.calls(), 0);
                            assert_eq!(executor.acceptance_attempts(), 1);
                            let reads_after_preflight = provider_clock.reads();
                            host.operations
                                .retry_consultation_task(*retry, &mut executor)
                                .expect("returned resumed work retries through its exact owner");
                            assert_eq!(provider_clock.reads(), reads_after_preflight);
                            assert_eq!(provider.calls(), 0);
                            assert_eq!(executor.acceptance_attempts(), 2);
                            let accepted = executor.take_accepted();
                            let mut output = None;
                            let _accepted = accepted.map_work(|work| {
                                let (work, completed) = ResidentProcessHost::<
                                UnprovedHeldWriter,
                            >::execute_consultation_child_provider_work(work, &provider);
                                output = Some(completed);
                                work
                            });
                            assert!(output.is_some());
                            assert_eq!(provider.calls(), 1);
                        });
                        return;
                    }
                    let mut executor = DeferredConsultChildExecutor::default();
                    let offered_dispatch = {
                        let mut admit_request =
                        |reference: &pale_core::records::ResidentRecordRefV1,
                         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                            let expected = if reference == &child.request_ref {
                                &child.request
                            } else if reference
                                == child.parent_request.0.record_ref()
                            {
                                &child.parent_request.1
                            } else {
                                panic!("unexpected provider Consult request")
                            };
                            assert_eq!(
                                pale_core::canon::to_canonical_string(&wire)
                                    .expect("request wire encoding"),
                                pale_core::canon::to_canonical_string(expected)
                                    .expect("expected request encoding")
                            );
                            Ok(expected.clone())
                        };
                        let mut admit_handle_authority =
                                    |preimage: &pale_core::records::OperationHandlePreimageV1,
                                     recovered_parent: Option<
                                        &pale_core::records::OperationHandleV1,
                                    >| {
                                        if &preimage.operation_handle_id
                                            == child.operation.handle().1.operation_handle_id()
                                        {
                                            assert_eq!(
                                                recovered_parent
                                                    .expect("provider child retains parent")
                                                    .canonical_operation_handle_sha256(),
                                                parent.handle().1.canonical_operation_handle_sha256()
                                            );
                                        } else {
                                            assert_eq!(
                                                &preimage.operation_handle_id,
                                                parent.handle().1.operation_handle_id()
                                            );
                                            assert!(recovered_parent.is_none());
                                        }
                                        Ok(())
                                    };
                        let mut verify_handle_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut admit_checkpoint = |_: &_, wire| {
                            pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                &checkpoint_signing,
                                |_, _, _, _| Ok(()),
                            )
                        };
                        let mut resolve_observation_signer =
                            |_: &_, _: &_| Ok(parents.signing.clone());
                        let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
                        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
                        let mut verify_observation_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut verify_checkpoint_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut recovery_admission = OperationControlRecoveryAdmission {
                            admit_request: &mut admit_request,
                            admit_handle_authority: &mut admit_handle_authority,
                            verify_handle_signature: &mut verify_handle_signature,
                            admit_checkpoint: &mut admit_checkpoint,
                            resolve_observation_signer: &mut resolve_observation_signer,
                            admit_observation_state: &mut admit_observation_state,
                            admit_rotation: &mut admit_rotation,
                            verify_observation_signature: &mut verify_observation_signature,
                            verify_checkpoint_signature: &mut verify_checkpoint_signature,
                        };
                        let open_restart = restart();
                        let mut open_authority = ProvingAuthority;
                        open_provider_consult_child(
                            host,
                            &selector,
                            ResidentProcessConsultProviderOpenTurn {
                                association: native_recovery_association(
                                    parents.signing,
                                    &open_restart,
                                ),
                                access: ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                authority: &mut open_authority,
                                task_limits,
                                observed_at,
                                current: current.clone(),
                                planned: batch,
                                candidate: candidate.clone(),
                                parent: parent_start,
                                recovered,
                                restart: Box::new(|| native_recovery_input(recovery())),
                                admission: &mut recovery_admission,
                                dispatch,
                                invocation,
                                executor: &mut executor,
                                rotate: Box::new(|_, _, _, _| Ok(())),
                                sign_observation: Box::new(|_, _, _| Ok(signature())),
                                sign_checkpoint: Box::new(|_, _, _| Ok(signature())),
                            },
                        )
                    };
                    assert_eq!(provider.calls(), 0, "retention precedes call");

                    let recovery_driver = host
                        .consultation_child_driver(&selector)
                        .expect("provider Consult recovery driver");
                    let dispatched = classify(
                        &recovery_driver,
                        custody,
                        anchor,
                        parent_start,
                        current,
                        batch,
                        candidate,
                        Some(offered_dispatch.dispatch()),
                        1,
                        observed_at,
                    )
                    .expect("durable provider dispatch recovers");
                    drop(recovery_driver);
                    let ResidentProcessOperationRestartDisposition::Provider(dispatched) =
                        dispatched
                    else {
                        panic!("provider Consult child recovers as dispatched")
                    };

                    let accepted = executor.take_accepted();
                    if matches!(
                        path,
                        ProviderConsultChildPath::Fresh | ProviderConsultChildPath::BoundedUnknown
                    ) {
                        let history = OperationAttemptHistoryInput {
                            operation: &child.operation,
                            request: &child.request,
                            session: child.operation.historical_session_lease(),
                            lease: child.operation.historical_operation_lease(),
                            generation,
                            dispatched: dispatched.start.control.observation().selected().1,
                            selection: &selection_record,
                            maximum_selection_bytes: 65_536,
                        };
                        let mut final_admit_request =
                            |reference: &pale_core::records::ResidentRecordRefV1,
                             wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                                let expected = if reference == &child.request_ref {
                                    &child.request
                                } else if reference == child.parent_request.0.record_ref() {
                                    &child.parent_request.1
                                } else {
                                    panic!("unexpected bounded provider Consult request")
                                };
                                assert_eq!(
                                    pale_core::canon::to_canonical_string(&wire)
                                        .expect("request wire encoding"),
                                    pale_core::canon::to_canonical_string(expected)
                                        .expect("expected request encoding")
                                );
                                Ok(expected.clone())
                            };
                        let mut final_admit_handle_authority =
                            |preimage: &pale_core::records::OperationHandlePreimageV1,
                             recovered_parent: Option<&pale_core::records::OperationHandleV1>| {
                                if &preimage.operation_handle_id
                                    == child.operation.handle().1.operation_handle_id()
                                {
                                    assert_eq!(
                                        recovered_parent
                                            .expect("provider child retains parent")
                                            .canonical_operation_handle_sha256(),
                                        parent.handle().1.canonical_operation_handle_sha256()
                                    );
                                } else {
                                    assert_eq!(
                                        &preimage.operation_handle_id,
                                        parent.handle().1.operation_handle_id()
                                    );
                                    assert!(recovered_parent.is_none());
                                }
                                Ok(())
                            };
                        let mut final_verify_handle_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut final_admit_checkpoint = |_: &_, wire| {
                            pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                &checkpoint_signing,
                                |_, _, _, _| Ok(()),
                            )
                        };
                        let mut final_resolve_observation_signer =
                            |_: &_, _: &_| Ok(parents.signing.clone());
                        let mut final_admit_observation_state =
                            |_: &OperationObservationStateV1| Ok(());
                        let mut final_admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
                        let mut final_verify_observation_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut final_verify_checkpoint_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut final_recovery_admission = OperationControlRecoveryAdmission {
                            admit_request: &mut final_admit_request,
                            admit_handle_authority: &mut final_admit_handle_authority,
                            verify_handle_signature: &mut final_verify_handle_signature,
                            admit_checkpoint: &mut final_admit_checkpoint,
                            resolve_observation_signer: &mut final_resolve_observation_signer,
                            admit_observation_state: &mut final_admit_observation_state,
                            admit_rotation: &mut final_admit_rotation,
                            verify_observation_signature: &mut final_verify_observation_signature,
                            verify_checkpoint_signature: &mut final_verify_checkpoint_signature,
                        };
                        let final_restart = restart();

                        let parent_predecessor = parent_start.control.observation().selected();
                        let parent_checkpoint = parent_start
                            .control
                            .observation()
                            .selected_checkpoint_record();
                        let child_dispatch_checkpoint =
                            dispatched.start.control.observation().current_checkpoint();
                        let mut join_authority_source = checkpoint_preimage_at_store(
                            child_dispatch_checkpoint.1.domain_sequence().get() + 2,
                            Some((
                                child_dispatch_checkpoint.0.clone(),
                                child_dispatch_checkpoint
                                    .1
                                    .canonical_control_checkpoint_sha256()
                                    .clone(),
                            )),
                            &selected_store,
                        );
                        join_authority_source.checkpoint_domain_id =
                            child_dispatch_checkpoint.1.checkpoint_domain_id().clone();
                        join_authority_source.checkpoint_profile_ref =
                            child_dispatch_checkpoint.1.checkpoint_profile_ref().clone();
                        join_authority_source.checkpoint_profile_sha256 = child_dispatch_checkpoint
                            .1
                            .checkpoint_profile_sha256()
                            .clone();
                        join_authority_source.domain_epoch =
                            *child_dispatch_checkpoint.1.domain_epoch();
                        let join_authority = checkpoint_authority(&join_authority_source);
                        let mut join_custody = CollectionCommitPort {
                            prepare: prepare_collection_accumulator,
                            transaction: ControlTransactionSequenceV1::new(
                                child_dispatch_checkpoint
                                    .1
                                    .committed_transaction_sequence()
                                    .get()
                                    + 2,
                            )
                                .expect("bounded join transaction"),
                            physical: crate::resident::operation_observation_custody::tests::RecordingStore::committing(
                                OperationCustodyCommitOutcome::Committed,
                            ),
                        };
                        let mut join_anchor = local_start::successor_anchor();
                        let resolver = SelectedStore::available(selected_store.clone());
                        let mut join_authority_port = ProvingAuthority;
                        let recording_calls = Rc::new(Cell::new(0));
                        let post_poll = ResidentProcessConsultProviderPostPoll {
                            access: ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody,
                                sessions: &sessions,
                                anchor,
                            },
                            history,
                            recovered_dispatch: dispatched.start.control.observation(),
                            lost_input: &lost_invocation,
                            evidence: &Evidence::valid(),
                            recovery: ResidentProcessConsultProviderRecordedRecovery {
                                association: native_recovery_association(
                                    parents.signing,
                                    &final_restart,
                                ),
                                restart: Box::new(|| native_recovery_input(recovery())),
                                admission: &mut final_recovery_admission,
                                parent: parent_start,
                                planned: batch,
                                observed_at,
                            },
                            recording: accepting_provider_recording_attempt(Rc::clone(
                                &recording_calls,
                            )),
                            parent: ResidentProcessConsultProviderParentTurn {
                                association: ResidentOperationControlAssociation {
                                    current_request_authority: parents.signing,
                                    historical_handle_authority: parents.signing,
                                    recovery_store: &selected_store,
                                    checkpoint_authority: &join_authority,
                                },
                                access: ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody: &mut join_custody,
                                    sessions: &sessions,
                                    anchor: &mut join_anchor,
                                },
                                authority: &mut join_authority_port,
                                slot_id: candidate.slot_id(),
                                parent,
                                parent_observation: parent_predecessor,
                                observation_ref: record_ref(
                                    "process-consult-provider/bounded-parent-joined",
                                ),
                                signing: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &observation_schema,
                                    signing_authority: parents.signing,
                                },
                                limits: ConsultationCollectionLimits::new(1, 65_536, 65_536)
                                    .expect("bounded collection limits"),
                                checkpoint_accumulator_limits: AccumulatorPreparationLimits::new(
                                    1, 65_536, 65_536, 8_192, 200_000,
                                )
                                .expect("bounded join accumulator limits"),
                                checkpoint_identity: OperationCheckpointInput {
                                    reference: record_ref(
                                        "process-consult-provider/bounded-parent-checkpoint",
                                    ),
                                    id: ControlCheckpointIdV1::parse(
                                        "process-consult-provider-bounded-parent",
                                    )
                                    .expect("bounded parent checkpoint identity"),
                                },
                                checkpoint_schema: &checkpoint_fields.schema_sha256,
                                checkpoint_authority: &join_authority,
                                observe_join_time: Box::new(move |terminal_child| {
                                    let child_observation = terminal_child.control.observation();
                                    let child_floor = child_observation
                                        .selected_head()
                                        .1
                                        .observed_at()
                                        .max(
                                            child_observation
                                                .selected_checkpoint_record()
                                                .1
                                                .issued_at(),
                                        )
                                        .get();
                                    let parent_floor = parent_predecessor
                                        .1
                                        .observed_at()
                                        .max(parent_checkpoint.1.issued_at())
                                        .get();
                                    UnixTimeV1::new(child_floor.max(parent_floor) + 1)
                                        .expect("bounded provider join time")
                                }),
                                resolve_store: &resolver,
                                admit_evidence: Box::new(|slot, _| {
                                    assert_eq!(slot.slot_id(), candidate.slot_id());
                                    Ok(())
                                }),
                                admit_collection: Box::new(|_| Ok(())),
                                rotate: Box::new(|_, _, _, _| Ok(())),
                                sign_observation: Box::new(|_, _, _| Ok(signature())),
                                sign_checkpoint: Box::new(|_, _, _| Ok(signature())),
                            },
                        };
                        let polls_before_pending = executor.completion_attempts();
                        let pending_turn = ResidentProcessConsultProviderChildTurn {
                            poll: ResidentProcessConsultProviderPollTurn {
                                recovered: &dispatched,
                                task_limits,
                                observed_at: UnixTimeV1::new(observed_at.get() + 1)
                                    .expect("empty provider completion poll time"),
                                current: current.clone(),
                                executor: &mut executor,
                            },
                            post_poll,
                        };
                        let calls_before_pending = provider.calls();
                        let pending = host
                            .continue_consultation_provider_child(&selector, pending_turn)
                            .unwrap_or_else(|_| panic!("empty provider poll stays pending"));
                        let continuation = match pending {
                            ResidentProcessConsultProviderChildDisposition::Pending {
                                continuation,
                                ..
                            } => *continuation,
                            _ => panic!("empty provider poll returns Pending"),
                        };
                        assert_eq!(provider.calls(), calls_before_pending);
                        assert_eq!(recording_calls.get(), 0);
                        assert_eq!(
                            executor.completion_attempts(),
                            polls_before_pending + 1,
                            "one bounded turn polls the executor exactly once"
                        );

                        if path == ProviderConsultChildPath::BoundedUnknown {
                            executor.lose(accepted);
                            assert_eq!(provider.calls(), 0);
                        } else {
                            let mut output = None;
                            let accepted = accepted.map_work(|work| {
                                let (work, completed) = ResidentProcessHost::<
                                    UnprovedHeldWriter,
                                >::execute_consultation_child_provider_work(work, &provider);
                                output = Some(completed);
                                work
                            });
                            executor.complete(accepted, output.expect("provider returned once"));
                            assert_eq!(provider.calls(), 1);
                        }

                        let polls_before_foreign_store = executor.completion_attempts();
                        let exact_turn = ResidentProcessConsultProviderChildTurn {
                            poll: ResidentProcessConsultProviderPollTurn {
                                recovered: &dispatched,
                                task_limits,
                                observed_at: UnixTimeV1::new(observed_at.get() + 2)
                                    .expect("ready provider completion poll time"),
                                current: current.clone(),
                                executor: &mut executor,
                            },
                            post_poll: continuation,
                        };
                        let ResidentProcessConsultProviderChildTurn {
                            poll,
                            post_poll:
                                ResidentProcessConsultProviderPostPoll {
                                    access,
                                    history,
                                    recovered_dispatch,
                                    lost_input,
                                    evidence,
                                    recovery,
                                    recording,
                                    parent,
                                },
                        } = exact_turn;
                        let ResidentDeploymentOperationAccess {
                            store: _,
                            custody: completion_custody,
                            sessions: completion_sessions,
                            anchor: completion_anchor,
                        } = access;
                        let foreign_turn = ResidentProcessConsultProviderChildTurn {
                            poll,
                            post_poll: ResidentProcessConsultProviderPostPoll {
                                access: ResidentDeploymentOperationAccess {
                                    store: &foreign_store,
                                    custody: completion_custody,
                                    sessions: completion_sessions,
                                    anchor: completion_anchor,
                                },
                                history,
                                recovered_dispatch,
                                lost_input,
                                evidence,
                                recovery,
                                recording,
                                parent,
                            },
                        };
                        let refused = match host
                            .continue_consultation_provider_child(&selector, foreign_turn)
                        {
                            Err(refused) => refused,
                            Ok(_) => panic!("foreign completion store refuses before polling"),
                        };
                        let ResidentProcessConsultProviderChildTurn {
                            poll,
                            post_poll:
                                ResidentProcessConsultProviderPostPoll {
                                    access,
                                    history,
                                    recovered_dispatch,
                                    lost_input,
                                    evidence,
                                    recovery,
                                    recording,
                                    parent,
                                },
                        } = match *refused {
                            ResidentProcessConsultProviderChildTurnFailure::Selection {
                                turn,
                                ..
                            } => *turn,
                            _ => panic!("foreign completion store preserves the whole turn"),
                        };
                        assert_eq!(
                            poll.executor.completion_attempts(),
                            polls_before_foreign_store,
                            "foreign completion store cannot consume an executor result"
                        );
                        let ResidentDeploymentOperationAccess {
                            store: _,
                            custody: completion_custody,
                            sessions: completion_sessions,
                            anchor: completion_anchor,
                        } = access;
                        let exact_turn = ResidentProcessConsultProviderChildTurn {
                            poll,
                            post_poll: ResidentProcessConsultProviderPostPoll {
                                access: ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody: completion_custody,
                                    sessions: completion_sessions,
                                    anchor: completion_anchor,
                                },
                                history,
                                recovered_dispatch,
                                lost_input,
                                evidence,
                                recovery,
                                recording,
                                parent,
                            },
                        };
                        let mut wrong_selector = selector.clone();
                        wrong_selector.principal_id =
                            pale_core::records::ResidentPrincipalIdV1::parse(
                                "process-consult-provider-bounded-foreign-principal",
                            )
                            .expect("foreign principal");
                        let refused = match host
                            .continue_consultation_provider_child(&wrong_selector, exact_turn)
                        {
                            Err(refused) => refused,
                            Ok(_) => panic!("foreign selector refuses before polling"),
                        };
                        let exact_turn = match *refused {
                            ResidentProcessConsultProviderChildTurnFailure::Selection {
                                turn,
                                ..
                            } => *turn,
                            _ => panic!("foreign selector preserves the whole bounded turn"),
                        };
                        assert_eq!(
                            provider.calls(),
                            usize::from(path == ProviderConsultChildPath::Fresh) as u32
                        );
                        assert_eq!(recording_calls.get(), 0);
                        assert_eq!(
                            exact_turn.poll.executor.completion_attempts(),
                            polls_before_foreign_store
                        );

                        let completed = match host
                            .continue_consultation_provider_child(&selector, exact_turn)
                        {
                            Ok(completed) => completed,
                            Err(failure) => match *failure {
                                ResidentProcessConsultProviderChildTurnFailure::Selection {
                                    reason,
                                    ..
                                } => panic!("bounded provider selection failed: {reason:?}"),
                                ResidentProcessConsultProviderChildTurnFailure::Poll { .. } => {
                                    panic!("bounded provider poll required recovery")
                                }
                                ResidentProcessConsultProviderChildTurnFailure::Completion {
                                    failure,
                                    ..
                                } => panic!("bounded provider finalization failed: {failure:?}"),
                                ResidentProcessConsultProviderChildTurnFailure::Join(failure) => {
                                    let phase = match *failure {
                                        ResidentProcessConsultationTerminalJoinFailure::Selection {
                                            ..
                                        } => "selection",
                                        ResidentProcessConsultationTerminalJoinFailure::Preflight {
                                            reason,
                                            ..
                                        } => panic!(
                                            "bounded provider parent join preflight failed: {reason:?}"
                                        ),
                                        ResidentProcessConsultationTerminalJoinFailure::Admission {
                                            reason,
                                            ..
                                        } => panic!(
                                            "bounded provider parent join admission failed: {reason:?}"
                                        ),
                                        ResidentProcessConsultationTerminalJoinFailure::Custody(
                                            reason,
                                        ) => panic!(
                                            "bounded provider parent join custody failed: {reason:?}"
                                        ),
                                        ResidentProcessConsultationTerminalJoinFailure::CustodyPreflight {
                                            ..
                                        } => "custody preflight",
                                        ResidentProcessConsultationTerminalJoinFailure::CustodySelection {
                                            ..
                                        } => "custody selection",
                                        ResidentProcessConsultationTerminalJoinFailure::Release {
                                            ..
                                        } => "release",
                                        ResidentProcessConsultationTerminalJoinFailure::ReleasePreflight {
                                            ..
                                        } => "release preflight",
                                        ResidentProcessConsultationTerminalJoinFailure::ReleaseSelection {
                                            ..
                                        } => "release selection",
                                    };
                                    panic!("bounded provider parent join failed during {phase}")
                                }
                            },
                        };
                        if path == ProviderConsultChildPath::BoundedUnknown {
                            let ResidentProcessConsultProviderChildDisposition::Nonterminal(
                                completed,
                            ) = completed
                            else {
                                panic!("durable unknown evidence settles without a parent join")
                            };
                            assert!(matches!(
                                completed.outcome(),
                                ResidentConsultationTaskPoll::Lost(_)
                            ));
                            assert_eq!(recording_calls.get(), 1);
                            assert_eq!(provider.calls(), 0, "lost child is never replayed");
                            return;
                        }
                        let ResidentProcessConsultProviderChildDisposition::Joined(completed) =
                            completed
                        else {
                            panic!("conclusive provider evidence joins its parent slot")
                        };
                        assert!(matches!(
                            completed.outcome(),
                            ResidentConsultationTaskPoll::Completed {
                                output: ResidentProcessConsultProviderChildOutput::Recorded(
                                    ProviderOperationInvokeResult::Completed(_),
                                ),
                                ..
                            }
                        ));
                        let joined_current = recover_completed_collection(completed.collection());
                        assert!(matches!(
                            joined_current.collection().slots()[0].disposition(),
                            ConsultationSlotDisposition::Unfulfillable { .. }
                        ));
                        assert_eq!(recording_calls.get(), 1);
                        assert_eq!(provider.calls(), 1, "bounded completion never replays");
                        return;
                    }
                    if path.is_lost_completion() {
                        executor.lose(accepted);
                        assert_eq!(provider.calls(), 0);
                    } else {
                        let mut output = None;
                        let accepted = accepted.map_work(|work| {
                            let (work, completed) =
                                ResidentProcessHost::<UnprovedHeldWriter>::
                                    execute_consultation_child_provider_work(
                                        work, &provider,
                                    );
                            output = Some(completed);
                            work
                        });
                        executor.complete(accepted, output.expect("provider returned once"));
                        assert_eq!(provider.calls(), 1);
                    }

                    let ready = poll_provider_consult_child(
                        host,
                        &selector,
                        ResidentProcessConsultProviderPollTurn {
                            recovered: &dispatched,
                            task_limits,
                            observed_at: UnixTimeV1::new(observed_at.get() + 2)
                                .expect("provider completion poll time"),
                            current: current.clone(),
                            executor: &mut executor,
                        },
                    );

                    let history = OperationAttemptHistoryInput {
                        operation: &child.operation,
                        request: &child.request,
                        session: child.operation.historical_session_lease(),
                        lease: child.operation.historical_operation_lease(),
                        generation,
                        dispatched: dispatched.start.control.observation().selected().1,
                        selection: &selection_record,
                        maximum_selection_bytes: 65_536,
                    };
                    if matches!(path, ProviderConsultChildPath::CompletionSettlement) {
                        record_recovered_provider_operation_unknown(
                            history,
                            dispatched.start.control.observation(),
                            &lost_invocation,
                            custody,
                            anchor,
                            |_| Ok(()),
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                            |_, _, _| Ok(signature()),
                        )
                        .expect("lost provider completion records durable uncertainty");
                        let driver = host
                            .consultation_child_driver(&selector)
                            .expect("provider settlement classification driver");
                        let classified = classify(
                            &driver,
                            custody,
                            anchor,
                            parent_start,
                            current,
                            batch,
                            candidate,
                            Some(offered_dispatch.dispatch()),
                            1,
                            observed_at,
                        )
                        .expect("durable provider uncertainty classifies");
                        drop(driver);
                        let ResidentProcessOperationRestartDisposition::RecoveryOnly(child) =
                            classified
                        else {
                            panic!("durable provider uncertainty remains recovery-only")
                        };
                        let expected_child_head =
                            child.control.observation().selected_head().0.clone();
                        let bound = host
                            .bind_consultation_completion(selector.clone(), *ready)
                            .unwrap_or_else(|_| panic!("provider settlement completion binds"));
                        let mut driver = host
                            .consultation_child_driver(&selector)
                            .expect("provider settlement driver");
                        let failure = match driver.settle_consultation_provider_nonterminal(
                            bound,
                            dispatched.start.control.observation(),
                        ) {
                            Err(failure) => failure,
                            Ok(_) => {
                                panic!("stale dispatched child refuses uncertainty settlement")
                            }
                        };
                        drop(driver);
                        assert_eq!(
                            child.control.observation().selected_head().0,
                            &expected_child_head
                        );
                        let continuation =
                            ResidentProcessConsultProviderCompletionContinuation::settlement(
                                failure, child,
                            );
                        assert_provider_completion_phase(&continuation, path);
                        let turn = provider_completion_callback_free_turn(continuation);
                        let turn =
                            preserve_provider_completion_on_wrong_selector(host, &selector, turn);
                        let completion = match host
                            .continue_consultation_provider_completion(&selector, turn)
                        {
                            Ok(completion) => completion,
                            Err(_) => panic!("exact classified child releases Consult capacity"),
                        };
                        let ResidentProcessConsultProviderCompletionDisposition::Nonterminal(
                            completed,
                        ) = completion
                        else {
                            panic!("settlement retry releases Consult capacity")
                        };
                        assert!(matches!(
                            completed.outcome(),
                            ResidentConsultationTaskPoll::Lost(_)
                        ));
                        assert_eq!(provider.calls(), 0, "settlement never calls the provider");
                        return;
                    }
                    let completion = {
                        let mut final_admit_request =
                        |reference: &pale_core::records::ResidentRecordRefV1,
                         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                            let expected = if reference == &child.request_ref {
                                &child.request
                            } else if reference
                                == child.parent_request.0.record_ref()
                            {
                                &child.parent_request.1
                            } else {
                                panic!("unexpected provider Consult request")
                            };
                            assert_eq!(
                                pale_core::canon::to_canonical_string(&wire)
                                    .expect("request wire encoding"),
                                pale_core::canon::to_canonical_string(expected)
                                    .expect("expected request encoding")
                            );
                            Ok(expected.clone())
                        };
                        let mut final_admit_handle_authority =
                                    |preimage: &pale_core::records::OperationHandlePreimageV1,
                                     recovered_parent: Option<
                                        &pale_core::records::OperationHandleV1,
                                    >| {
                                        if &preimage.operation_handle_id
                                            == child.operation.handle().1.operation_handle_id()
                                        {
                                            assert_eq!(
                                                recovered_parent
                                                    .expect("provider child retains parent")
                                                    .canonical_operation_handle_sha256(),
                                                parent.handle().1.canonical_operation_handle_sha256()
                                            );
                                        } else {
                                            assert_eq!(
                                                &preimage.operation_handle_id,
                                                parent.handle().1.operation_handle_id()
                                            );
                                            assert!(recovered_parent.is_none());
                                        }
                                        Ok(())
                                    };
                        let mut final_verify_handle_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut final_admit_checkpoint = |_: &_, wire| {
                            pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                &checkpoint_signing,
                                |_, _, _, _| Ok(()),
                            )
                        };
                        let mut final_resolve_observation_signer =
                            |_: &_, _: &_| Ok(parents.signing.clone());
                        let mut final_admit_observation_state =
                            |_: &OperationObservationStateV1| Ok(());
                        let mut final_admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
                        let mut final_verify_observation_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut final_verify_checkpoint_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut final_recovery_admission = OperationControlRecoveryAdmission {
                            admit_request: &mut final_admit_request,
                            admit_handle_authority: &mut final_admit_handle_authority,
                            verify_handle_signature: &mut final_verify_handle_signature,
                            admit_checkpoint: &mut final_admit_checkpoint,
                            resolve_observation_signer: &mut final_resolve_observation_signer,
                            admit_observation_state: &mut final_admit_observation_state,
                            admit_rotation: &mut final_admit_rotation,
                            verify_observation_signature: &mut final_verify_observation_signature,
                            verify_checkpoint_signature: &mut final_verify_checkpoint_signature,
                        };
                        let final_restart = restart();
                        let mut classification_refusals = 2;
                        let initial_recording_calls = Rc::new(Cell::new(0));
                        let turn = ResidentProcessConsultProviderFinalizeTurn {
                            access: ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody,
                                sessions: &sessions,
                                anchor,
                            },
                            ready: *ready,
                            history,
                            recovered_dispatch: dispatched.start.control.observation(),
                            lost_input: &lost_invocation,
                            evidence: &Evidence::valid(),
                            recovery: ResidentProcessConsultProviderRecordedRecovery {
                                association: native_recovery_association(
                                    parents.signing,
                                    &final_restart,
                                ),
                                restart: Box::new(|| {
                                    let mut input = native_recovery_input(recovery());
                                    if matches!(
                                        path,
                                        ProviderConsultChildPath::CompletionClassification
                                    ) && classification_refusals > 0
                                    {
                                        classification_refusals -= 1;
                                        input.maximum_parent_depth = 0;
                                    }
                                    input
                                }),
                                admission: &mut final_recovery_admission,
                                parent: parent_start,
                                planned: batch,
                                observed_at,
                            },
                            recording: if matches!(
                                path,
                                ProviderConsultChildPath::CompletionRecording
                                    | ProviderConsultChildPath::CompletionLostRecording
                            ) {
                                rejecting_provider_recording_attempt(Rc::clone(
                                    &initial_recording_calls,
                                ))
                            } else {
                                accepting_provider_recording_attempt(Rc::clone(
                                    &initial_recording_calls,
                                ))
                            },
                        };
                        match path {
                            ProviderConsultChildPath::CompletionRecording
                            | ProviderConsultChildPath::CompletionLostRecording => {
                                let continuation = finalize_provider_consult_child_continuation(
                                    host, &selector, turn,
                                );
                                assert_provider_completion_phase(&continuation, path);
                                assert_eq!(initial_recording_calls.get(), 1);

                                let retry_recording_calls = Rc::new(Cell::new(0));
                                let turn = provider_completion_recording_turn(
                                    continuation,
                                    if matches!(
                                        path,
                                        ProviderConsultChildPath::CompletionLostRecording
                                    ) {
                                        rejecting_provider_checkpoint_attempt(Rc::clone(
                                            &retry_recording_calls,
                                        ))
                                    } else {
                                        rejecting_provider_recording_attempt(Rc::clone(
                                            &retry_recording_calls,
                                        ))
                                    },
                                );
                                let turn = preserve_provider_completion_on_wrong_selector(
                                    host, &selector, turn,
                                );
                                assert_eq!(
                                    retry_recording_calls.get(),
                                    0,
                                    "wrong selector preserves untouched one-shot callbacks"
                                );
                                let continuation = continue_provider_completion_expect_retry(
                                    host, &selector, turn,
                                );
                                assert_provider_completion_phase(&continuation, path);
                                assert_eq!(retry_recording_calls.get(), 1);
                                let continuation =
                                    assert_provider_completion_requires_fresh_recording(
                                        continuation,
                                    );

                                let successful_recording_calls = Rc::new(Cell::new(0));
                                let turn = if matches!(
                                    path,
                                    ProviderConsultChildPath::CompletionLostRecording
                                ) {
                                    provider_completion_checkpoint_turn(
                                        continuation,
                                        Rc::clone(&successful_recording_calls),
                                    )
                                } else {
                                    provider_completion_recording_turn(
                                        continuation,
                                        accepting_provider_recording_attempt(Rc::clone(
                                            &successful_recording_calls,
                                        )),
                                    )
                                };
                                let completion = match host
                                    .continue_consultation_provider_completion(&selector, turn)
                                {
                                    Ok(completion) => completion,
                                    Err(_) => panic!(
                                        "fresh provider recording callbacks complete without replay"
                                    ),
                                };
                                assert_eq!(successful_recording_calls.get(), 1);
                                completion
                            }
                            ProviderConsultChildPath::CompletionClassification => {
                                let continuation = finalize_provider_consult_child_continuation(
                                    host, &selector, turn,
                                );
                                assert_provider_completion_phase(&continuation, path);
                                assert_eq!(initial_recording_calls.get(), 1);
                                let turn = provider_completion_callback_free_turn(continuation);
                                let turn = preserve_provider_completion_on_wrong_selector(
                                    host, &selector, turn,
                                );
                                let continuation = continue_provider_completion_expect_retry(
                                    host, &selector, turn,
                                );
                                assert_provider_completion_phase(&continuation, path);
                                let turn = provider_completion_callback_free_turn(continuation);
                                match host
                                    .continue_consultation_provider_completion(&selector, turn)
                                {
                                    Ok(completion) => completion,
                                    Err(_) => panic!(
                                        "provider classification continuation succeeds without replay"
                                    ),
                                }
                            }
                            _ => {
                                let completion =
                                    finalize_provider_consult_child(host, &selector, turn);
                                assert_eq!(initial_recording_calls.get(), 1);
                                completion
                            }
                        }
                    };
                    let expected_provider_calls = usize::from(!path.is_lost_completion()) as u32;
                    assert_eq!(
                        provider.calls(),
                        expected_provider_calls,
                        "finalization never replays"
                    );
                    if matches!(path, ProviderConsultChildPath::CompletionLostRecording) {
                        assert!(matches!(
                            completion,
                            ResidentProcessConsultProviderCompletionDisposition::Nonterminal(_)
                        ));
                        return;
                    }

                    let child_join_floor = match &completion {
                        ResidentProcessConsultProviderCompletionDisposition::Terminal {
                            child,
                            ..
                        } => child
                            .control
                            .observation()
                            .selected_head()
                            .1
                            .observed_at()
                            .max(
                                child
                                    .control
                                    .observation()
                                    .selected_checkpoint_record()
                                    .1
                                    .issued_at(),
                            )
                            .get(),
                        ResidentProcessConsultProviderCompletionDisposition::Nonterminal(_) => {
                            panic!("conclusive provider failure became nonterminal")
                        }
                    };
                    let child_checkpoint = match &completion {
                        ResidentProcessConsultProviderCompletionDisposition::Terminal {
                            child,
                            ..
                        } => child.control.observation().current_checkpoint(),
                        ResidentProcessConsultProviderCompletionDisposition::Nonterminal(_) => {
                            unreachable!("conclusive provider completion was checked above")
                        }
                    };
                    let parent_predecessor = parent_start.control.observation().selected();
                    let parent_checkpoint = parent_start
                        .control
                        .observation()
                        .selected_checkpoint_record();
                    let parent_join_floor = parent_predecessor
                        .1
                        .observed_at()
                        .max(parent_checkpoint.1.issued_at())
                        .get();
                    let join_time = UnixTimeV1::new(child_join_floor.max(parent_join_floor) + 1)
                        .expect("parent join time");
                    let mut join_authority_source = checkpoint_preimage_at_store(
                        child_checkpoint.1.domain_sequence().get() + 1,
                        Some((
                            child_checkpoint.0.clone(),
                            child_checkpoint
                                .1
                                .canonical_control_checkpoint_sha256()
                                .clone(),
                        )),
                        &selected_store,
                    );
                    join_authority_source.checkpoint_domain_id =
                        child_checkpoint.1.checkpoint_domain_id().clone();
                    join_authority_source.checkpoint_profile_ref =
                        child_checkpoint.1.checkpoint_profile_ref().clone();
                    join_authority_source.checkpoint_profile_sha256 =
                        child_checkpoint.1.checkpoint_profile_sha256().clone();
                    join_authority_source.domain_epoch = *child_checkpoint.1.domain_epoch();
                    let join_authority = checkpoint_authority(&join_authority_source);
                    let mut join_custody = CollectionCommitPort {
                        prepare: prepare_collection_accumulator,
                        transaction: ControlTransactionSequenceV1::new(
                            child_checkpoint.1.committed_transaction_sequence().get() + 1,
                        )
                            .expect("join transaction"),
                        physical: crate::resident::operation_observation_custody::tests::RecordingStore::committing(
                            OperationCustodyCommitOutcome::Committed,
                        ),
                    };
                    let mut join_anchor = local_start::successor_anchor();
                    let resolver = SelectedStore::available(selected_store.clone());
                    let completed = join_provider_consult_child(
                        host,
                        ResidentProcessConsultProviderJoinTurn {
                            association: ResidentOperationControlAssociation {
                                current_request_authority: parents.signing,
                                historical_handle_authority: parents.signing,
                                recovery_store: &selected_store,
                                checkpoint_authority: &join_authority,
                            },
                            access: ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody: &mut join_custody,
                                sessions: &sessions,
                                anchor: &mut join_anchor,
                            },
                            authority: &mut ProvingAuthority,
                            completion,
                            input: ResidentProcessConsultProviderParentJoinInput {
                                slot_id: candidate.slot_id(),
                                recorded_at: join_time,
                                parent,
                                parent_observation: parent_predecessor,
                                observation_ref: record_ref(
                                    "process-consult-provider/parent-joined",
                                ),
                                observed_at: join_time,
                                signing: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &observation_schema,
                                    signing_authority: parents.signing,
                                },
                                limits: ConsultationCollectionLimits::new(1, 65_536, 65_536)
                                    .expect("collection bounds"),
                            },
                            checkpoint: ConsultationSuccessorCommitInput {
                                accumulator_limits: AccumulatorPreparationLimits::new(
                                    1, 65_536, 65_536, 8_192, 200_000,
                                )
                                .expect("join accumulator bounds"),
                                identity: OperationCheckpointInput {
                                    reference: record_ref(
                                        "process-consult-provider/parent-joined-checkpoint",
                                    ),
                                    id: ControlCheckpointIdV1::parse(
                                        "process-consult-provider-parent-joined",
                                    )
                                    .expect("parent join checkpoint identity"),
                                },
                                issued_at: join_time,
                                schema: &checkpoint_fields.schema_sha256,
                                authority: &join_authority,
                            },
                            resolve_store: &resolver,
                            admit_evidence: Box::new(|slot, _| {
                                assert_eq!(slot.slot_id(), candidate.slot_id());
                                Ok(())
                            }),
                            admit_collection: Box::new(|_| Ok(())),
                            rotate: Box::new(|_, _, _, _| Ok(())),
                            sign_observation: Box::new(|_, _, _| Ok(signature())),
                            sign_checkpoint: Box::new(|_, _, _| Ok(signature())),
                        },
                    );
                    assert!(matches!(
                        completed.outcome(),
                        ResidentConsultationTaskPoll::Completed {
                            output: ResidentProcessConsultProviderChildOutput::Recorded(
                                ProviderOperationInvokeResult::Completed(_),
                            ),
                            ..
                        }
                    ));
                    let joined_current = recover_completed_collection(completed.collection());
                    assert!(matches!(
                        joined_current.collection().slots()[0].disposition(),
                        ConsultationSlotDisposition::Unfulfillable { .. }
                    ));
                    assert_eq!(provider.calls(), 1);
                },
            );
        },
    );
}

fn run_native_consult_child_tail(
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) {
    let ConsultChildFixture {
        child,
        parent,
        generation: _,
        parents,
        ..
    } = fixture;
    let RecoveredConsultChild {
        parent_start,
        current,
        batch,
        candidate,
        recovered,
        observed_at,
    } = recovered_child;
    let selected_store = child.initial_candidate.store().clone();
    let selector = selector_from_signing_authority(parents.signing);
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let child_lineage = recovered.start.lineage.clone();
    let session = parents.session.lease();
    let sessions = RestartSessions(SessionLeaseBody::new(
        selected_store.clone(),
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                .expect("session wire")
                .into_bytes(),
        )
        .expect("bounded session wire"),
    ));
    let mut historical = RecoveryResolver::default();
    historical.insert_generation(parents.current.generation.compiled_generation);
    historical.insert_active(parents.current.generation.active_head);
    historical.insert_fence(parents.current.generation.activation_fence);
    historical.insert_effective_profile(parents.current.generation.effective_profile);
    let child_handle = crate::resident::record_projection::coordinate(
        child.operation.handle().0,
        child
            .operation
            .handle()
            .1
            .canonical_operation_handle_sha256()
            .as_str(),
    )
    .expect("child recovery coordinate");
    let observation_limits =
        OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20).expect("observation bounds");
    let checkpoint_limits =
        OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).expect("checkpoint bounds");
    let membership_limits =
        AccumulatorMembershipLimits::new(8, 20_000, 200_000).expect("membership bounds");
    let checkpoint_profile = crate::local_operation_start_fixture::profile();
    let mut checkpoint_fields = checkpoint_preimage_at_store(
        child.checkpoint.domain_sequence().get() + 1,
        Some((
            child.checkpoint_ref.clone(),
            child
                .checkpoint
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
        &selected_store,
    );
    checkpoint_fields.checkpoint_domain_id = child.checkpoint.checkpoint_domain_id().clone();
    checkpoint_fields.checkpoint_profile_ref = child.checkpoint.checkpoint_profile_ref().clone();
    checkpoint_fields.checkpoint_profile_sha256 =
        child.checkpoint.checkpoint_profile_sha256().clone();
    checkpoint_fields.domain_epoch = *child.checkpoint.domain_epoch();
    let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
    let recovery = || OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: &selected_store,
                profile: &checkpoint_profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority: &checkpoint_signing,
                historical: &historical,
                maximum_record_bytes: 1 << 20,
                maximum_operation_lease_bytes: 1 << 20,
                maximum_session_lease_bytes: 1 << 20,
                membership_limits,
                observation_limits,
                checkpoint_limits,
            },
            expected_handle: &child_handle,
            maximum_parent_depth: 1,
        },
        observation: OperationObservationSelection::SelectedHead,
        observation_limits,
        checkpoint_limits,
        membership_limits,
    };
    let restart = || native_recovery_input(recovery());

    let classify = |driver: &ResidentProcessConsultChildDriver<'_, '_>,
                    custody: &mut dyn pale_ports::OperationCustodyPort,
                    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
                    selected_parent: &ResidentProcessRecoveredOperationStart,
                    selected_current: &RecoveredConsultationCollection,
                    selected_batch: &ConsultationChildDispatchBatch,
                    selected_candidate: &ConsultationChildDispatchCandidate,
                    selected_dispatch: Option<&ImmutableRecordReferenceV1>,
                    maximum_parent_depth: usize,
                    selected_observed_at: UnixTimeV1| {
        let mut admit_request =
            |reference: &pale_core::records::ResidentRecordRefV1,
             wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                let expected = if reference == &child.request_ref {
                    &child.request
                } else if reference == child.parent_request.0.record_ref() {
                    &child.parent_request.1
                } else {
                    panic!("unexpected Consult-child request reference")
                };
                assert_eq!(
                    pale_core::canon::to_canonical_string(&wire).expect("request wire encoding"),
                    pale_core::canon::to_canonical_string(expected)
                        .expect("expected request encoding")
                );
                Ok(expected.clone())
            };
        let mut admit_handle_authority =
            |preimage: &pale_core::records::OperationHandlePreimageV1,
             recovered_parent: Option<&pale_core::records::OperationHandleV1>| {
                if &preimage.operation_handle_id == child.operation.handle().1.operation_handle_id()
                {
                    let recovered_parent =
                        recovered_parent.expect("child recovery retains its Consult parent");
                    assert_eq!(
                        recovered_parent.canonical_operation_handle_sha256(),
                        parent.handle().1.canonical_operation_handle_sha256()
                    );
                } else {
                    assert_eq!(
                        &preimage.operation_handle_id,
                        parent.handle().1.operation_handle_id()
                    );
                    assert!(recovered_parent.is_none());
                }
                Ok(())
            };
        let mut verify_handle_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
        let mut admit_checkpoint = |_: &_, wire| {
            pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                wire,
                &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                &checkpoint_signing,
                |_, _, _, _| Ok(()),
            )
        };
        let mut resolve_observation_signer = |_: &_, _: &_| Ok(parents.signing.clone());
        let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
        let mut verify_observation_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
        let mut verify_checkpoint_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
        let mut selected_restart = restart();
        selected_restart.maximum_parent_depth = maximum_parent_depth;
        driver.classify_child_operation(
            native_recovery_association(parents.signing, &selected_restart),
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions: &sessions,
                anchor,
            },
            selected_restart,
            &mut OperationControlRecoveryAdmission {
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
            selected_parent,
            selected_current,
            selected_batch,
            selected_candidate,
            selected_dispatch,
            selected_observed_at,
        )
    };
    let registry = NativeActionRegistry::new().expect("sealed native registry");
    let attempt_id =
        CapabilityAttemptIdV1::parse("process-consult-native-attempt").expect("attempt identity");
    let binding = child
        .request
        .local_request_binding()
        .expect("local request binding");
    let contracts = crate::session_admission_fixture::local_start_contract_set();
    let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
    let kind =
        pale_core::records::BoundedResidentTextV1::parse(CapabilityProvider::NativeRust.as_str())
            .expect("native implementation kind");
    let version = pale_core::records::BoundedResidentTextV1::parse(
        registry
            .contract(pale_core::records::PATH_INVENTORY_ACTION_ID)
            .expect("path inventory contract")
            .version
            .to_string(),
    )
    .expect("native implementation version");
    let admission = CapabilitySuccessAdmissionContextV1 {
        result: CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: &result_schema,
            expected_request_binding: &binding,
            expected_attempt_id: &attempt_id,
            expected_implementation_kind: &kind,
            expected_implementation_version: &version,
            signing_authority: parents.signing,
        },
        contract_set_ref: binding.contract_set_ref(),
        contract_set: &contracts,
    };
    let recovery_floor = std::cmp::max(
        recovered
            .start
            .control
            .observation()
            .selected_head()
            .1
            .observed_at()
            .get(),
        child.checkpoint.issued_at().get(),
    );
    let clock = IncreasingClock::starting_at(recovery_floor + 1, None);
    let resolver = SelectedStore::available(selected_store.clone());
    let checkpoint_input = |name: &str| OperationCheckpointInput {
        reference: record_ref(&format!("process-consult-native/{name}-checkpoint")),
        id: ControlCheckpointIdV1::parse(format!("process-consult-native-{name}"))
            .expect("checkpoint identity"),
    };
    let signs = Cell::new(0_usize);
    let checkpoint_signs = Cell::new(0_usize);
    let resolution = |current| ResidentProcessOperationResolutionInput {
        session: parents.session,
        current,
        signing: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &observation_schema,
            signing_authority: parents.signing,
        },
        selection_ref: record_ref("process-consult-native/selection"),
        observation_ref: record_ref("process-consult-native/preparation"),
        maximum_selection_bytes: 65_536,
        maximum_record_bytes: 65_536,
    };
    let basis = || ResidentProcessNativeInvokeBasis {
        registry: &registry,
        attempt_id: &attempt_id,
        result_admission: admission,
        checkpoint_schema: &checkpoint_fields.schema_sha256,
        checkpoint_authority: &checkpoint_signing,
        preparation_checkpoint: checkpoint_input("prepared"),
        dispatch_checkpoint: checkpoint_input("dispatched"),
        outcome_checkpoint: checkpoint_input("outcome"),
        terminal_checkpoint: checkpoint_input("terminal"),
        dispatch_ref: record_ref("process-consult-native/dispatched"),
        outcome_ref: record_ref("process-consult-native/outcome"),
        outcome_observation_ref: record_ref("process-consult-native/outcome-observation"),
        result_ref: record_ref("process-consult-native/result"),
        diagnostic_ref: record_ref("process-consult-native/diagnostic"),
        terminal_ref: record_ref("process-consult-native/terminal"),
        clock: &clock,
        resolve_store: &resolver,
        maximum_diagnostic_bytes: 65_536,
        maximum_outcome_bytes: 65_536,
        accumulator_limits: AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
            .expect("accumulator limits"),
    };
    let foreign = owner_native_signing_authority(selector.principal_id.clone());
    let admitted_current = parents.current;
    let foreign_current = crate::resident::AmbientSessionAdmissionContext {
        now: admitted_current.now,
        subject: admitted_current.subject,
        grant: admitted_current.grant,
        current_principal_trust: admitted_current.current_principal_trust,
        capability_revocations: admitted_current.capability_revocations,
        deployment_binding_ref: admitted_current.deployment_binding_ref,
        deployment_binding: admitted_current.deployment_binding,
        deployment_revocations: admitted_current.deployment_revocations,
        runtime_authority: admitted_current.runtime_authority,
        session_revocations: admitted_current.session_revocations,
        route_use: admitted_current.route_use,
        generation: pale_core::records::SessionAdmissionLeaseContextV1 {
            active_head: admitted_current.generation.active_head,
            compiled_generation: admitted_current.generation.compiled_generation,
            effective_profile: admitted_current.generation.effective_profile,
            activation_fence: admitted_current.generation.activation_fence,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: admitted_current.generation.signed.expected_schema_sha256,
                signing_authority: &foreign,
            },
        },
    };
    let broadened =
        ResidentProcessHost::<UnprovedHeldWriter>::prepare_consultation_child_native_work_in(
            host.deployments[0].application.as_ref(),
            recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions: &sessions,
                anchor,
            },
            resolution(&foreign_current),
            basis(),
            &mut ProvingAuthority,
            |_, _, _, _| Ok(()),
            |_, _, _| panic!("broadened authority must not sign"),
            |_, _, _| panic!("broadened authority must not sign a checkpoint"),
        );
    assert!(broadened.is_err());
    let mut driver = host
        .consultation_child_driver(&selector)
        .expect("Consult-child deployment driver after authority rejection");
    let mut executor = DeferredConsultChildExecutor::default();
    let task_limits = ResidentConsultationTaskLimits::new(
        NonZeroUsize::new(1).expect("one child in flight"),
        NonZeroUsize::new(1).expect("one roster member"),
    );
    let mut admit_request =
        |reference: &pale_core::records::ResidentRecordRefV1,
         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
            let expected = if reference == &child.request_ref {
                &child.request
            } else if reference == child.parent_request.0.record_ref() {
                &child.parent_request.1
            } else {
                panic!("unexpected Consult-child request reference")
            };
            assert_eq!(
                pale_core::canon::to_canonical_string(&wire).expect("request wire encoding"),
                pale_core::canon::to_canonical_string(expected).expect("expected request encoding")
            );
            Ok(expected.clone())
        };
    let mut admit_handle_authority =
        |preimage: &pale_core::records::OperationHandlePreimageV1,
         recovered_parent: Option<&pale_core::records::OperationHandleV1>| {
            if &preimage.operation_handle_id == child.operation.handle().1.operation_handle_id() {
                assert_eq!(
                    recovered_parent
                        .expect("child retains its Consult parent")
                        .canonical_operation_handle_sha256(),
                    parent.handle().1.canonical_operation_handle_sha256()
                );
            } else {
                assert_eq!(
                    &preimage.operation_handle_id,
                    parent.handle().1.operation_handle_id()
                );
                assert!(recovered_parent.is_none());
            }
            Ok(())
        };
    let mut verify_handle_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut admit_checkpoint = |_: &_, wire| {
        pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
            wire,
            &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
            &checkpoint_signing,
            |_, _, _, _| Ok(()),
        )
    };
    let mut resolve_observation_signer = |_: &_, _: &_| Ok(parents.signing.clone());
    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
    let mut verify_observation_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut verify_checkpoint_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut supervisor_restart = || native_recovery_input(recovery());
    let offered = driver
        .supervise_consultation_native_child_turn(
            native_recovery_association(parents.signing, &restart()),
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions: &sessions,
                anchor,
            },
            &mut ProvingAuthority,
            task_limits,
            observed_at,
            current.clone(),
            batch,
            candidate.clone(),
            parent_start,
            recovered,
            &mut supervisor_restart,
            &mut OperationControlRecoveryAdmission {
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
            resolution(parents.current),
            basis(),
            &mut executor,
            |_, _, _, _| Ok(()),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
            |_, _, _| {
                checkpoint_signs.set(checkpoint_signs.get() + 1);
                Ok(signature())
            },
        )
        .expect("task owner offers the durably dispatched native child");
    let ResidentConsultationTurnDisposition::Progress(ResidentConsultationTurnProgress::Offered {
        dispatch: offered_dispatch,
        ..
    }) = offered
    else {
        panic!("the task owner receives the durable child dispatch")
    };
    assert_eq!(signs.get(), 2);
    assert_eq!(checkpoint_signs.get(), 2);

    let accepted = executor.take_accepted();
    let mut output = None;
    let accepted = accepted.map_work(|work| {
        let (work, completed) =
            ResidentProcessHost::<UnprovedHeldWriter>::execute_consultation_child_native_work(
                work,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions: &sessions,
                    anchor,
                },
                &mut ProvingAuthority,
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    signs.set(signs.get() + 1);
                    Ok(signature())
                },
                |_, _, _| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    Ok(signature())
                },
                |_, _, _| Ok(signature()),
            );
        output = Some(completed);
        work
    });
    let output = output.expect("task-owned native handler returned once");
    assert!(output
        .as_ref()
        .expect("Consult-child native Invoke completes")
        .result()
        .is_ok());
    executor.complete(accepted, output);
    assert_eq!(signs.get(), 4);
    assert_eq!(checkpoint_signs.get(), 4);

    let ready = driver.poll_consultation_native_child_turn(
        task_limits,
        UnixTimeV1::new(observed_at.get() + 2).expect("completion poll time"),
        current.clone(),
        &mut executor,
    );
    let ResidentConsultationTurnDisposition::Progress(
        ResidentConsultationTurnProgress::CompletionReady(ready),
    ) = ready
    else {
        panic!("the task owner retains the exact native completion")
    };

    let terminal_child = match classify(
        &driver,
        custody,
        anchor,
        parent_start,
        current,
        batch,
        candidate,
        Some(offered_dispatch.dispatch()),
        1,
        observed_at,
    )
    .expect("protected recovery admits the completed Consult child")
    {
        ResidentProcessOperationRestartDisposition::Terminal(recovered) => recovered,
        other => panic!("completed Consult child recovers as terminal: {other:?}"),
    };
    assert!(child_lineage.admits(
        terminal_child.control.operation().operation().handle().0,
        terminal_child.control.operation().operation().handle().1,
    ));
    assert!(matches!(
        parent_start.control.observation().selected().1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::CollectingConsultation { .. }
        }
    ));

    let collection_limits =
        ConsultationCollectionLimits::new(1, 65_536, 65_536).expect("collection bounds");
    let parent_predecessor = parent_start.control.observation().selected();
    let parent_checkpoint = parent_start
        .control
        .observation()
        .selected_checkpoint_record();
    let child_checkpoint = terminal_child.control.observation().current_checkpoint();
    let mut join_authority_source = checkpoint_preimage_at_store(
        child_checkpoint.1.domain_sequence().get() + 1,
        Some((
            child_checkpoint.0.clone(),
            child_checkpoint
                .1
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
        &selected_store,
    );
    join_authority_source.checkpoint_domain_id = child_checkpoint.1.checkpoint_domain_id().clone();
    join_authority_source.checkpoint_profile_ref =
        child_checkpoint.1.checkpoint_profile_ref().clone();
    join_authority_source.checkpoint_profile_sha256 =
        child_checkpoint.1.checkpoint_profile_sha256().clone();
    join_authority_source.domain_epoch = *child_checkpoint.1.domain_epoch();
    let join_authority = checkpoint_authority(&join_authority_source);
    let mut join_custody = CollectionCommitPort {
        prepare: prepare_collection_accumulator,
        transaction: ControlTransactionSequenceV1::new(
            child_checkpoint.1.committed_transaction_sequence().get() + 1,
        )
        .expect("join transaction"),
        physical: crate::resident::operation_observation_custody::tests::RecordingStore::committing(
            OperationCustodyCommitOutcome::Committed,
        ),
    };
    let mut join_anchor = local_start::successor_anchor();
    let bound = driver
        .bind_completion(*ready)
        .unwrap_or_else(|_| panic!("completion binds to its exact deployment"));
    let completed_join = driver.join_terminal(
        ResidentOperationControlAssociation {
            current_request_authority: parents.signing,
            historical_handle_authority: parents.signing,
            recovery_store: &selected_store,
            checkpoint_authority: &join_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut join_custody,
            sessions: &sessions,
            anchor: &mut join_anchor,
        },
        &mut ProvingAuthority,
        bound,
        ConsultationChildJoinInput {
            slot_id: candidate.slot_id(),
            child: terminal_child.control.observation(),
            recorded_at: UnixTimeV1::new(observed_at.get() + 10).expect("parent join time"),
            parent,
            parent_observation: parent_predecessor,
            observation_ref: record_ref("process-consult-native/parent-joined"),
            observed_at: UnixTimeV1::new(observed_at.get() + 10)
                .expect("parent join observation time"),
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority: parents.signing,
            },
            limits: collection_limits,
        },
        ConsultationSuccessorCommitInput {
            accumulator_limits: AccumulatorPreparationLimits::new(
                1, 65_536, 65_536, 8_192, 200_000,
            )
            .expect("parent join accumulator bounds"),
            identity: checkpoint_input("parent-joined"),
            issued_at: UnixTimeV1::new(observed_at.get() + 10)
                .expect("parent join checkpoint time"),
            schema: &checkpoint_fields.schema_sha256,
            authority: &join_authority,
        },
        &resolver,
        |slot, evidence| {
            assert_eq!(slot.slot_id(), candidate.slot_id());
            assert_eq!(
                evidence.record_ref,
                record_ref("process-consult-native/result")
            );
            Ok(())
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
    );
    let completed_join = expect_consult_child_terminal_join(completed_join);
    assert!(matches!(
        completed_join.outcome(),
        ResidentConsultationTaskPoll::Completed { output: Ok(completed), .. }
            if completed.result().is_ok()
    ));
    let joined_parent = completed_join.collection();
    assert!(matches!(
        joined_parent.observation().1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::CollectingConsultation { .. }
        }
    ));
    let joined_current = recover_completed_collection(joined_parent);
    assert!(matches!(
        joined_current.collection().slots()[0].disposition(),
        ConsultationSlotDisposition::Fulfilled { evidence, .. }
            if evidence.record_ref
                == record_ref("process-consult-native/result")
    ));

    let joined_checkpoint = joined_parent.checkpoint();
    let mut stop_authority_source = checkpoint_preimage_at_store(
        joined_checkpoint.1.domain_sequence().get() + 1,
        Some((
            joined_checkpoint.0.clone(),
            joined_checkpoint
                .1
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
        &selected_store,
    );
    stop_authority_source.checkpoint_domain_id = joined_checkpoint.1.checkpoint_domain_id().clone();
    stop_authority_source.checkpoint_profile_ref =
        joined_checkpoint.1.checkpoint_profile_ref().clone();
    stop_authority_source.checkpoint_profile_sha256 =
        joined_checkpoint.1.checkpoint_profile_sha256().clone();
    stop_authority_source.domain_epoch = *joined_checkpoint.1.domain_epoch();
    let stop_authority = checkpoint_authority(&stop_authority_source);
    let mut stop_custody = CollectionCommitPort {
        prepare: prepare_collection_accumulator,
        transaction: ControlTransactionSequenceV1::new(
            joined_checkpoint.1.committed_transaction_sequence().get() + 1,
        )
        .expect("stop transaction"),
        physical: crate::resident::operation_observation_custody::tests::RecordingStore::committing(
            OperationCustodyCommitOutcome::Committed,
        ),
    };
    let mut stop_anchor = local_start::successor_anchor();
    let stopped_parent = ResidentConsultationCoordinator::new(&mut stop_custody, &mut stop_anchor)
        .stop(
            &joined_current,
            ConsultationStopInput {
                cause: ConsultationStopCause::CompletionPolicy,
                reason: ImmutableRecordReferenceV1 {
                    record_ref: record_ref("process-consult-native/completion-policy"),
                    record_sha256: ArtifactDigest::hash_bytes(b"completion-policy"),
                },
                stopped_at: UnixTimeV1::new(observed_at.get() + 11).expect("parent stop time"),
                parent,
                parent_observation: joined_parent.observation(),
                parent_checkpoint: joined_parent.checkpoint(),
                observation_ref: record_ref("process-consult-native/parent-stopped"),
                observed_at: UnixTimeV1::new(observed_at.get() + 11)
                    .expect("parent stop observation time"),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: parents.signing,
                },
                limits: collection_limits,
            },
            ConsultationSuccessorCommitInput {
                accumulator_limits: AccumulatorPreparationLimits::new(
                    1, 65_536, 65_536, 8_192, 200_000,
                )
                .expect("parent stop accumulator bounds"),
                identity: checkpoint_input("parent-stopped"),
                issued_at: UnixTimeV1::new(observed_at.get() + 11)
                    .expect("parent stop checkpoint time"),
                schema: &checkpoint_fields.schema_sha256,
                authority: &stop_authority,
            },
            &resolver,
            |_, summary| {
                assert!(summary.requirements_met());
                Ok(())
            },
            |_| Ok(()),
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
            |_, _, _| Ok(signature()),
        )
        .expect("joined parent stops through its existing policy owner");
    let stopped_current = recover_completed_collection(&stopped_parent);
    assert!(stopped_current
        .collection()
        .stopped()
        .is_some_and(|stop| stop.summary().requirements_met()));
    assert!(matches!(
        stopped_parent.observation().1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::ConsultationCollected { .. }
        }
    ));

    let mut early_stop_custody = CollectionCommitPort {
        prepare: prepare_collection_accumulator,
        transaction: ControlTransactionSequenceV1::new(2).expect("early stop transaction"),
        physical: crate::resident::operation_observation_custody::tests::RecordingStore::committing(
            OperationCustodyCommitOutcome::Committed,
        ),
    };
    let mut early_stop_anchor = local_start::successor_anchor();
    let early_stopped_parent =
        ResidentConsultationCoordinator::new(&mut early_stop_custody, &mut early_stop_anchor)
            .stop(
                current,
                ConsultationStopInput {
                    cause: ConsultationStopCause::CompletionPolicy,
                    reason: ImmutableRecordReferenceV1 {
                        record_ref: record_ref("process-consult-native/early-stop-policy"),
                        record_sha256: ArtifactDigest::hash_bytes(b"early-stop-policy"),
                    },
                    stopped_at: UnixTimeV1::new(observed_at.get() + 9)
                        .expect("early parent stop time"),
                    parent,
                    parent_observation: parent_predecessor,
                    parent_checkpoint,
                    observation_ref: record_ref("process-consult-native/parent-stopped-early"),
                    observed_at: UnixTimeV1::new(observed_at.get() + 9)
                        .expect("early parent stop observation time"),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: parents.signing,
                    },
                    limits: collection_limits,
                },
                ConsultationSuccessorCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 65_536, 65_536, 8_192, 200_000,
                    )
                    .expect("early parent stop accumulator bounds"),
                    identity: checkpoint_input("parent-stopped-early"),
                    issued_at: UnixTimeV1::new(observed_at.get() + 9)
                        .expect("early parent stop checkpoint time"),
                    schema: &checkpoint_fields.schema_sha256,
                    authority: &join_authority,
                },
                &resolver,
                |_, summary| {
                    assert_eq!(summary.required_outstanding().len(), 1);
                    Ok(())
                },
                |_| Ok(()),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_, _, _| Ok(signature()),
            )
            .expect("parent can stop before its child finishes");
    let early_stopped_current = recover_completed_collection(&early_stopped_parent);
    let frozen_summary = early_stopped_current
        .collection()
        .stopped()
        .expect("early stop retained")
        .summary()
        .clone();
    let admitted_late = driver
        .admit_child_terminal(
            parent_start,
            &early_stopped_current,
            &terminal_child,
            UnixTimeV1::new(observed_at.get() + 10).expect("late child record time"),
            collection_limits,
            |slot, evidence| {
                assert_eq!(slot.slot_id(), candidate.slot_id());
                assert_eq!(
                    evidence.record_ref,
                    record_ref("process-consult-native/result")
                );
                Ok(())
            },
        )
        .expect("lineaged terminal evidence remains retainable after stop");
    assert_eq!(
        admitted_late
            .stopped()
            .expect("late successor remains stopped")
            .summary(),
        &frozen_summary
    );
    assert_eq!(admitted_late.late_outcomes().len(), 1);
    assert!(matches!(
        admitted_late.slots()[0].disposition(),
        ConsultationSlotDisposition::Outstanding
    ));

    let mut late_join_authority_source = checkpoint_preimage_at_store(
        child_checkpoint.1.domain_sequence().get() + 1,
        Some((
            child_checkpoint.0.clone(),
            child_checkpoint
                .1
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
        &selected_store,
    );
    late_join_authority_source.checkpoint_domain_id =
        child_checkpoint.1.checkpoint_domain_id().clone();
    late_join_authority_source.checkpoint_profile_ref =
        child_checkpoint.1.checkpoint_profile_ref().clone();
    late_join_authority_source.checkpoint_profile_sha256 =
        child_checkpoint.1.checkpoint_profile_sha256().clone();
    late_join_authority_source.domain_epoch = *child_checkpoint.1.domain_epoch();
    let late_join_authority = checkpoint_authority(&late_join_authority_source);
    let mut late_join_custody = CollectionCommitPort {
        prepare: prepare_collection_accumulator,
        transaction: ControlTransactionSequenceV1::new(
            child_checkpoint.1.committed_transaction_sequence().get() + 1,
        )
        .expect("late join transaction"),
        physical: crate::resident::operation_observation_custody::tests::RecordingStore::committing(
            OperationCustodyCommitOutcome::Committed,
        ),
    };
    let mut late_join_anchor = local_start::successor_anchor();
    let retained_late =
        ResidentConsultationCoordinator::new(&mut late_join_custody, &mut late_join_anchor)
            .join_child(
                &early_stopped_current,
                ConsultationChildJoinInput {
                    slot_id: candidate.slot_id(),
                    child: terminal_child.control.observation(),
                    recorded_at: UnixTimeV1::new(observed_at.get() + 10)
                        .expect("late durable record time"),
                    parent,
                    parent_observation: early_stopped_parent.observation(),
                    observation_ref: record_ref("process-consult-native/parent-late-child"),
                    observed_at: UnixTimeV1::new(observed_at.get() + 10)
                        .expect("late parent observation time"),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: parents.signing,
                    },
                    limits: collection_limits,
                },
                ConsultationSuccessorCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 65_536, 65_536, 8_192, 200_000,
                    )
                    .expect("late join accumulator bounds"),
                    identity: checkpoint_input("parent-late-child"),
                    issued_at: UnixTimeV1::new(observed_at.get() + 10)
                        .expect("late join checkpoint time"),
                    schema: &checkpoint_fields.schema_sha256,
                    authority: &late_join_authority,
                },
                &resolver,
                |slot, evidence| {
                    assert_eq!(slot.slot_id(), candidate.slot_id());
                    assert_eq!(
                        evidence.record_ref,
                        record_ref("process-consult-native/result")
                    );
                    Ok(())
                },
                |_| Ok(()),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_, _, _| Ok(signature()),
            )
            .expect("late terminal evidence reaches durable parent custody");
    let retained_late = recover_completed_collection(&retained_late);
    assert_eq!(
        retained_late
            .collection()
            .stopped()
            .expect("late commit remains stopped")
            .summary(),
        &frozen_summary
    );
    assert_eq!(retained_late.collection().late_outcomes().len(), 1);
    assert!(matches!(
        retained_late.collection().slots()[0].disposition(),
        ConsultationSlotDisposition::Outstanding
    ));

    let reads_before_restart = clock.reads();
    let restarted = classify(
        &driver,
        custody,
        anchor,
        parent_start,
        current,
        batch,
        candidate,
        None,
        1,
        observed_at,
    )
    .expect("terminal Consult child remains recoverable");
    assert!(matches!(
        restarted,
        ResidentProcessOperationRestartDisposition::Terminal(_)
    ));
    assert_eq!(clock.reads(), reads_before_restart);
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ConsultChildPreDispatch {
    Continue,
    Cancel,
    Expire,
}

fn run_consult_child_native_invoke(pre_dispatch: ConsultChildPreDispatch) {
    run_consult_child_on_bounded_stack("consult-native", move || {
        run_consult_child_invoke(
            "process-consult-native",
            ConsultChildFixtureKind::SelfContained,
            pre_dispatch,
            4,
            4,
            fresh_provider_consult_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_invoke() {
    run_consult_child_on_bounded_stack("consult-provider-fresh", || {
        run_consult_child_invoke(
            "process-consult-provider",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            4,
            4,
            fresh_provider_consult_tail,
        );
    });
    run_consult_child_on_bounded_stack("consult-provider-bounded-unknown", || {
        run_consult_child_invoke(
            "process-consult-provider-bounded-unknown",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            4,
            4,
            bounded_unknown_provider_consult_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_resume() {
    run_consult_child_on_bounded_stack("consult-provider-same-process-resume", || {
        run_consult_child_invoke(
            "process-consult-provider-resume",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            3,
            3,
            same_process_provider_consult_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_recovered_reference_collision() {
    run_consult_child_on_bounded_stack("consult-provider-recovered-collision", || {
        run_consult_child_invoke(
            "process-consult-provider-reference-collision",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            3,
            3,
            colliding_recovered_provider_consult_tail,
        );
    });
}

fn run_consult_child_on_bounded_stack(name: &str, run: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .expect("bounded provider Consult test thread")
        .join()
        .expect("bounded provider Consult test completes");
}

pub(super) fn run_consult_child_provider_cold_current_head_loss() {
    run_consult_child_on_bounded_stack("consult-provider-cold-current-head", || {
        run_consult_child_invoke(
            "process-consult-provider-cold-loss",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            4,
            4,
            cold_provider::run_provider_consult_cold_current_head_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_cold_cancellation_predecessor_loss() {
    run_consult_child_on_bounded_stack("consult-provider-cold-cancellation", || {
        run_consult_child_invoke(
            "process-consult-provider-cold-cancellation-loss",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            5,
            5,
            cold_provider::run_provider_consult_cold_cancellation_predecessor_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_cold_recovered_reference_collision() {
    run_consult_child_on_bounded_stack("consult-provider-cold-collision", || {
        run_consult_child_invoke(
            "process-consult-provider-cold-collision",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            3,
            3,
            cold_provider::run_provider_consult_cold_collision_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_cold_offered_continuation() {
    run_consult_child_on_bounded_stack("consult-provider-cold-offered", || {
        run_consult_child_invoke(
            "process-consult-provider-cold-offered",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            4,
            4,
            cold_provider::run_provider_consult_cold_offered_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_recording_continuation() {
    run_consult_child_on_bounded_stack("consult-provider-recording-continuation", || {
        run_consult_child_invoke(
            "process-consult-provider-recording-continuation",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            4,
            4,
            recording_provider_consult_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_lost_recording_continuation() {
    run_consult_child_on_bounded_stack("consult-provider-lost-recording", || {
        run_consult_child_invoke(
            "process-consult-provider-lost-recording-continuation",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            5,
            4,
            lost_recording_provider_consult_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_classification_continuation() {
    run_consult_child_on_bounded_stack("consult-provider-classification", || {
        run_consult_child_invoke(
            "process-consult-provider-classification-continuation",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            4,
            4,
            classification_provider_consult_tail,
        );
    });
}

pub(super) fn run_consult_child_provider_settlement_continuation() {
    run_consult_child_on_bounded_stack("consult-provider-settlement", || {
        run_consult_child_invoke(
            "process-consult-provider-settlement-continuation",
            ConsultChildFixtureKind::ProviderBacked,
            ConsultChildPreDispatch::Continue,
            4,
            4,
            settlement_provider_consult_tail,
        );
    });
}

fn run_consult_child_invoke(
    label: &'static str,
    kind: ConsultChildFixtureKind,
    pre_dispatch: ConsultChildPreDispatch,
    provider_expected_preparations: usize,
    provider_expected_commits: usize,
    provider_tail: ProviderConsultChildTail,
) {
    with_process_consult_invoke_child(
        label,
        kind,
        |child,
         parent,
         parent_candidate,
         parent_checkpoint,
         lease_custody,
         generation,
         parents,
         host_generation| {
            let selected_store = child.initial_candidate.store().clone();
            let selector = selector_from_signing_authority(parents.signing);
            let child_coordinate = ImmutableRecordReferenceV1 {
                record_ref: child.operation.handle().0.clone(),
                record_sha256: ArtifactDigest::parse(
                    child
                        .operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .expect("child handle digest"),
            };
            let (provider_binding, provider_participant) =
                if kind == ConsultChildFixtureKind::ProviderBacked {
                    let provider = generation
                        .provider_binding("provider-binding-claude-code")
                        .expect("compiled provider binding");
                    (
                        Some(ProviderBindingRefV1 {
                            provider_binding_id: provider.id.clone(),
                            provider_binding_identity: provider.binding_identity.clone(),
                        }),
                        Some(provider.participant.clone()),
                    )
                } else {
                    (None, None)
                };
            let current = if kind == ConsultChildFixtureKind::ProviderBacked {
                consultation_supervision_fixture_for_provider_operation_in_store(
                    parent.handle().0,
                    parent.handle().1,
                    label,
                    child_coordinate.clone(),
                    provider_participant
                        .clone()
                        .expect("compiled provider participant"),
                    provider_binding
                        .clone()
                        .expect("compiled provider binding reference"),
                    selected_store.clone(),
                )
            } else {
                consultation_supervision_fixture_for_self_contained_operation_in_store(
                    parent.handle().0,
                    parent.handle().1,
                    label,
                    child_coordinate.clone(),
                    false,
                    selected_store.clone(),
                )
            };
            let observed_at = UnixTimeV1::new(parent.handle().1.created_at().get() + 1)
                .expect("child dispatch time");
            let batch = current
                .plan_child_dispatch(observed_at)
                .expect("one exact self-contained child");
            let candidate = batch.candidates()[0].clone();
            assert!(candidate.context().is_none());
            assert_eq!(candidate.provider_binding(), provider_binding.as_ref());

            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let parent_initial_record = parent_candidate
                .record(OperationCustodyRecordKind::OperationObservation)
                .expect("parent initial observation");
            let parent_initial_wire = serde_json::from_value(
                pale_core::canon::parse_strict(
                    parent_initial_record.canonical_record_jcs().as_slice(),
                )
                .expect("parent observation JSON"),
            )
            .expect("parent observation wire");
            let parent_initial = pale_core::records::OperationObservationV1::admit_contextual_wire(
                parent_initial_wire,
                &OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: parents.signing,
                    },
                    handle_ref: parent.handle().0,
                    handle: parent.handle().1,
                    predecessor: None,
                },
                |_| Ok(()),
                |_, _, _, _| Ok(()),
                |_, _, _, _| Ok(()),
            )
            .expect("parent initial observation admits");
            let parent_open_ref = record_ref("process-consult-native/parent-open");
            let parent_open = issue_observation(
                parents.signing,
                parent.handle().0,
                parent.handle().1,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(2).expect("open revision"),
                Some((
                    parent_initial_record.coordinate().record_ref(),
                    &parent_initial,
                )),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::CollectingConsultation {
                        evidence: batch.collection_frontier().clone(),
                    },
                },
                observed_at.get(),
            );
            let parent_handle_coordinate = crate::resident::record_projection::coordinate(
                parent.handle().0,
                parent
                    .handle()
                    .1
                    .canonical_operation_handle_sha256()
                    .as_str(),
            )
            .expect("parent handle coordinate");
            let parent_checkpoint_ref = parent_candidate
                .checkpoint()
                .successor_record()
                .coordinate()
                .record_ref()
                .clone();
            let recovered_parent_operation = crate::resident::RecoveredLocalOperation::fixture(
                parent,
                child.parent_request.1.clone(),
                selected_store.clone(),
            );
            let parent_observation =
                crate::resident::RecoveredOperationObservation::history_fixture_for_handle(
                    parent_handle_coordinate,
                    (parent_open_ref.clone(), parent_open.clone()),
                    Some((
                        parent_initial_record.coordinate().record_ref().clone(),
                        parent_initial.clone(),
                    )),
                    None,
                    (parent_open_ref, parent_open),
                    (parent_checkpoint_ref.clone(), parent_checkpoint.clone()),
                    selected_store.clone(),
                );
            let mut parent_start = ResidentProcessRecoveredOperationStart {
                selector: selector.clone(),
                control: crate::resident::RecoveredOperationControl::fixture(
                    recovered_parent_operation,
                    parent_observation,
                ),
                generation_digest: parent.handle().1.compiled_generation_sha256().clone(),
                lineage: ResidentProcessOperationLineage::Root,
            };

            let handle_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    child.operation.handle().0,
                    child
                        .operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .expect("child handle coordinate"),
                child.operation.handle().1,
            )
            .expect("child handle record");
            let expected_preparations = match pre_dispatch {
                ConsultChildPreDispatch::Cancel | ConsultChildPreDispatch::Expire => 1,
                ConsultChildPreDispatch::Continue => provider_expected_preparations,
            };
            let expected_commits = match pre_dispatch {
                ConsultChildPreDispatch::Cancel | ConsultChildPreDispatch::Expire => 1,
                ConsultChildPreDispatch::Continue => provider_expected_commits,
            };
            custody::with_service_custody_faults(
                &child.checkpoint,
                Some(&child.initial_candidate),
                vec![parent_candidate.clone()],
                None,
                vec![lease_custody.parent, lease_custody.child],
                expected_preparations,
                expected_commits,
                expected_commits,
                expected_commits,
                expected_commits,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                Vec::new(),
                false,
                Some(handle_record),
                Rc::new(RefCell::new(None)),
                |custody, anchor| {
                    let application_store = MemoryAuthorityStore::default();
                    let mut application =
                        ready_application(&application_store, selected_store.clone(), 81);
                    application.fixture_retain_generation(host_generation);
                    let deployment = unproved_deployment_for_selector(
                        &selector,
                        selected_store.clone(),
                        application,
                    );
                    let mut host = ResidentProcessHost::new(
                        unproved_writer(
                            selector.principal_id.clone(),
                            vec![selected_store.clone()],
                        ),
                        vec![deployment],
                        NonZeroUsize::new(1).expect("one deployment"),
                        ResidentOperationHost::new(task_state()),
                    )
                    .expect("Consult-child process host");
                    let driver = host
                        .consultation_child_driver(&selector)
                        .expect("Consult-child deployment driver");

                    let session = parents.session.lease();
                    let sessions = RestartSessions(SessionLeaseBody::new(
                        selected_store.clone(),
                        session.session_admission_lease_digest().clone(),
                        session.schema_id().clone(),
                        session.schema_sha256().clone(),
                        pale_ports::ControlRecordBytes::new(
                            pale_core::canon::to_canonical_string(
                                &SessionAdmissionLeaseWireV1::from(session),
                            )
                            .expect("session wire")
                            .into_bytes(),
                        )
                        .expect("bounded session wire"),
                    ));
                    let mut historical = RecoveryResolver::default();
                    historical.insert_generation(parents.current.generation.compiled_generation);
                    historical.insert_active(parents.current.generation.active_head);
                    historical.insert_fence(parents.current.generation.activation_fence);
                    historical
                        .insert_effective_profile(parents.current.generation.effective_profile);
                    let child_handle = crate::resident::record_projection::coordinate(
                        child.operation.handle().0,
                        child
                            .operation
                            .handle()
                            .1
                            .canonical_operation_handle_sha256()
                            .as_str(),
                    )
                    .expect("child recovery coordinate");
                    let observation_limits =
                        OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20)
                            .expect("observation bounds");
                    let checkpoint_limits = OperationCheckpointAncestryLimits::new(8, 1 << 20, 8)
                        .expect("checkpoint bounds");
                    let membership_limits = AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                        .expect("membership bounds");
                    let checkpoint_profile = crate::local_operation_start_fixture::profile();
                    let mut checkpoint_fields = checkpoint_preimage_at_store(
                        child.checkpoint.domain_sequence().get() + 1,
                        Some((
                            child.checkpoint_ref.clone(),
                            child
                                .checkpoint
                                .canonical_control_checkpoint_sha256()
                                .clone(),
                        )),
                        &selected_store,
                    );
                    checkpoint_fields.checkpoint_domain_id =
                        child.checkpoint.checkpoint_domain_id().clone();
                    checkpoint_fields.checkpoint_profile_ref =
                        child.checkpoint.checkpoint_profile_ref().clone();
                    checkpoint_fields.checkpoint_profile_sha256 =
                        child.checkpoint.checkpoint_profile_sha256().clone();
                    checkpoint_fields.domain_epoch = *child.checkpoint.domain_epoch();
                    let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
                    let recovery = || OperationControlRecoveryInput {
                        operation: LocalOperationHandleRecoveryInput {
                            context: LocalOperationRecoveryContext {
                                store: &selected_store,
                                profile: &checkpoint_profile,
                                handle_signing_authority: parents.signing,
                                checkpoint_authority: &checkpoint_signing,
                                historical: &historical,
                                maximum_record_bytes: 1 << 20,
                                maximum_operation_lease_bytes: 1 << 20,
                                maximum_session_lease_bytes: 1 << 20,
                                membership_limits,
                                observation_limits,
                                checkpoint_limits,
                            },
                            expected_handle: &child_handle,
                            maximum_parent_depth: 1,
                        },
                        observation: OperationObservationSelection::SelectedHead,
                        observation_limits,
                        checkpoint_limits,
                        membership_limits,
                    };
                    let restart = || native_recovery_input(recovery());

                    let classify =
                        |driver: &ResidentProcessConsultChildDriver<'_, '_>,
                         custody: &mut dyn pale_ports::OperationCustodyPort,
                         anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
                         selected_parent: &ResidentProcessRecoveredOperationStart,
                         selected_current: &RecoveredConsultationCollection,
                         selected_batch: &ConsultationChildDispatchBatch,
                         selected_candidate: &ConsultationChildDispatchCandidate,
                         selected_dispatch: Option<&ImmutableRecordReferenceV1>,
                         maximum_parent_depth: usize,
                         selected_observed_at: UnixTimeV1| {
                            let mut admit_request =
                            |reference: &pale_core::records::ResidentRecordRefV1,
                             wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                                let expected = if reference == &child.request_ref {
                                    &child.request
                                } else if reference == child.parent_request.0.record_ref() {
                                    &child.parent_request.1
                                } else {
                                    panic!("unexpected Consult-child request reference")
                                };
                                assert_eq!(
                                    pale_core::canon::to_canonical_string(&wire)
                                        .expect("request wire encoding"),
                                    pale_core::canon::to_canonical_string(expected)
                                        .expect("expected request encoding")
                                );
                                Ok(expected.clone())
                            };
                            let mut admit_handle_authority =
                                |preimage: &pale_core::records::OperationHandlePreimageV1,
                                 recovered_parent: Option<
                                    &pale_core::records::OperationHandleV1,
                                >| {
                                    if &preimage.operation_handle_id
                                        == child.operation.handle().1.operation_handle_id()
                                    {
                                        let recovered_parent = recovered_parent
                                            .expect("child recovery retains its Consult parent");
                                        assert_eq!(
                                            recovered_parent.canonical_operation_handle_sha256(),
                                            parent.handle().1.canonical_operation_handle_sha256()
                                        );
                                    } else {
                                        assert_eq!(
                                            &preimage.operation_handle_id,
                                            parent.handle().1.operation_handle_id()
                                        );
                                        assert!(recovered_parent.is_none());
                                    }
                                    Ok(())
                                };
                            let mut verify_handle_signature =
                                |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                            let mut admit_checkpoint = |_: &_, wire| {
                                pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                    &checkpoint_signing,
                                    |_, _, _, _| Ok(()),
                                )
                            };
                            let mut resolve_observation_signer =
                                |_: &_, _: &_| Ok(parents.signing.clone());
                            let mut admit_observation_state =
                                |_: &OperationObservationStateV1| Ok(());
                            let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
                            let mut verify_observation_signature =
                                |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                            let mut verify_checkpoint_signature =
                                |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                            let mut selected_restart = restart();
                            selected_restart.maximum_parent_depth = maximum_parent_depth;
                            driver.classify_child_operation(
                                native_recovery_association(parents.signing, &selected_restart),
                                ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                selected_restart,
                                &mut OperationControlRecoveryAdmission {
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
                                selected_parent,
                                selected_current,
                                selected_batch,
                                selected_candidate,
                                selected_dispatch,
                                selected_observed_at,
                            )
                        };

                    if pre_dispatch == ConsultChildPreDispatch::Cancel {
                        let cancellation_request = super::a2a_cancel_task::cancellation_request(
                            &child.request,
                            child.operation.handle().1,
                        );
                        let cancellation_time = cancellation_request.issued_at();
                        let mut admit_request =
                            |reference: &pale_core::records::ResidentRecordRefV1,
                             wire: pale_core::records::CommonRequestEnvelopeWireV1| {
                                let expected = if reference == &child.request_ref {
                                    &child.request
                                } else if reference == child.parent_request.0.record_ref() {
                                    &child.parent_request.1
                                } else {
                                    panic!("unexpected Consult-child cancellation request")
                                };
                                assert_eq!(
                                    pale_core::canon::to_canonical_string(&wire)
                                        .expect("request wire encoding"),
                                    pale_core::canon::to_canonical_string(expected)
                                        .expect("expected request encoding")
                                );
                                Ok(expected.clone())
                            };
                        let mut admit_handle_authority =
                            |preimage: &pale_core::records::OperationHandlePreimageV1,
                             recovered_parent: Option<&pale_core::records::OperationHandleV1>| {
                                if &preimage.operation_handle_id
                                    == child.operation.handle().1.operation_handle_id()
                                {
                                    assert_eq!(
                                        recovered_parent
                                            .expect("cancelled child retains its Consult parent")
                                            .canonical_operation_handle_sha256(),
                                        parent
                                            .handle()
                                            .1
                                            .canonical_operation_handle_sha256()
                                    );
                                } else {
                                    assert!(recovered_parent.is_none());
                                }
                                Ok(())
                            };
                        let mut verify_handle_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut admit_checkpoint = |_: &_, wire| {
                            pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                &checkpoint_signing,
                                |_, _, _, _| Ok(()),
                            )
                        };
                        let mut resolve_observation_signer =
                            |_: &_, _: &_| Ok(parents.signing.clone());
                        let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
                        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
                        let mut verify_observation_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let mut verify_checkpoint_signature =
                            |_: &[u8], _: &_, _: &_, _: &_| Ok(());
                        let recovered_control = recover_operation_control(
                            recovery(),
                            custody,
                            &sessions,
                            anchor,
                            OperationControlRecoveryAdmission {
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
                        .expect("Consult-child cancellation recovery");
                        let authorized = recovered_control
                            .authorize_cancellation(
                                &cancellation_request,
                                cancellation_time,
                                |_| Ok(()),
                            )
                            .expect("Consult-child cancellation authority");
                        let prepared = authorized
                            .prepare_local_request(
                                LocalOperationCancellationRequestInput {
                                    request_ref: record_ref(
                                        "process-consult-native/cancellation-request",
                                    ),
                                    observation_ref: record_ref(
                                        "process-consult-native/cancellation-requested",
                                    ),
                                    signing: SignedResidentAdmissionContextV1 {
                                        expected_schema_sha256: &observation_schema,
                                        signing_authority: parents.signing,
                                    },
                                },
                                |_, _, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                            )
                            .expect("Consult-child cancellation preparation");
                        let prior = prepared.predecessor_checkpoint();
                        let cancellation_resolver =
                            SelectedStore::available(selected_store.clone());
                        let prepared_checkpoint = prepared
                            .prepare_checkpoint(
                                AccumulatorPreparationLimits::new(
                                    8, 20_000, 20_000, 8_192, 200_000,
                                )
                                .expect("cancellation accumulator limits"),
                                &cancellation_resolver,
                                custody,
                            )
                            .expect("Consult-child cancellation checkpoint");
                        complete_prepared_checkpoint(
                            prepared_checkpoint,
                            prior,
                            OperationCheckpointInput {
                                reference: record_ref(
                                    "process-consult-native/cancellation-checkpoint",
                                ),
                                id: ControlCheckpointIdV1::parse(
                                    "process-consult-native-cancellation",
                                )
                                .expect("cancellation checkpoint identity"),
                            },
                            cancellation_time,
                            &checkpoint_fields.schema_sha256,
                            &checkpoint_signing,
                            custody,
                            anchor,
                            |_, _, _| Ok(signature()),
                        )
                        .expect("Consult-child cancellation custody");

                        let cancelled = classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &current,
                            &batch,
                            &candidate,
                            None,
                            1,
                            cancellation_time,
                        )
                        .expect("cancelled Consult child remains recoverable");
                        let ResidentProcessOperationRestartDisposition::LocalCancellationRequested(
                            cancelled,
                        ) = cancelled
                        else {
                            panic!("pre-dispatch cancellation wins before native work")
                        };
                        assert!(cancelled.start.lineage.consult_child().is_some());
                        assert!(matches!(
                            cancelled
                                .start
                                .control
                                .observation()
                                .selected_predecessor()
                                .expect("cancellation predecessor")
                                .1
                                .state(),
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving
                            }
                        ));
                        drop(driver);
                        assert_eq!(host.operations.outstanding_task_reservations(), 0);
                        return;
                    }

                    if kind == ConsultChildFixtureKind::SelfContained {
                        let provider_current =
                            consultation_supervision_fixture_for_provider_operation_in_store(
                                parent.handle().0,
                                parent.handle().1,
                                "process-consult-native",
                                child_coordinate.clone(),
                                pale_core::records::ParticipantRefV1::Application {
                                    application_id: "fixture-unadmitted-provider".into(),
                                },
                                ProviderBindingRefV1 {
                                    provider_binding_id: "fixture-unadmitted-provider".into(),
                                    provider_binding_identity: ProviderBindingIdentityV1 {
                                        provider_binding_sha256: ArtifactDigest::hash_bytes(
                                            b"fixture-unadmitted-provider",
                                        ),
                                    },
                                },
                                selected_store.clone(),
                            );
                        let provider_batch = provider_current
                            .plan_child_dispatch(observed_at)
                            .expect("provider-backed child remains schedulable");
                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &provider_current,
                            &provider_batch,
                            &provider_batch.candidates()[0],
                            None,
                            1,
                            observed_at,
                        )
                        .is_err());

                        let contextual_current =
                            consultation_supervision_fixture_for_operation_in_store(
                                parent.handle().0,
                                parent.handle().1,
                                "process-consult-native",
                                child_coordinate.clone(),
                                false,
                                selected_store.clone(),
                            );
                        let contextual_batch = contextual_current
                            .plan_child_dispatch(observed_at)
                            .expect("contextual child remains schedulable");
                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &contextual_current,
                            &contextual_batch,
                            &contextual_batch.candidates()[0],
                            None,
                            1,
                            observed_at,
                        )
                        .is_err());

                        let foreign_slot_current =
                            consultation_supervision_fixture_for_self_contained_operation_in_store(
                                parent.handle().0,
                                parent.handle().1,
                                "process-consult-native-foreign-slot",
                                child_coordinate.clone(),
                                false,
                                selected_store.clone(),
                            );
                        let foreign_slot_batch = foreign_slot_current
                            .plan_child_dispatch(observed_at)
                            .expect("foreign slot remains schedulable");
                        assert_ne!(
                            foreign_slot_batch.candidates()[0].slot_id(),
                            candidate.slot_id()
                        );
                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &foreign_slot_current,
                            &foreign_slot_batch,
                            &foreign_slot_batch.candidates()[0],
                            None,
                            1,
                            observed_at,
                        )
                        .is_err());

                        let foreign_child_coordinate = ImmutableRecordReferenceV1 {
                            record_ref: child_coordinate.record_ref.clone(),
                            record_sha256: ArtifactDigest::hash_bytes(b"foreign-child-handle"),
                        };
                        let foreign_child_current =
                            consultation_supervision_fixture_for_self_contained_operation_in_store(
                                parent.handle().0,
                                parent.handle().1,
                                "process-consult-native",
                                foreign_child_coordinate,
                                false,
                                selected_store.clone(),
                            );
                        let foreign_child_batch = foreign_child_current
                            .plan_child_dispatch(observed_at)
                            .expect("foreign child remains schedulable");
                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &foreign_child_current,
                            &foreign_child_batch,
                            &foreign_child_batch.candidates()[0],
                            None,
                            1,
                            observed_at,
                        )
                        .is_err());

                        let foreign_parent_current =
                            consultation_supervision_fixture_for_self_contained_operation_in_store(
                                child.operation.handle().0,
                                child.operation.handle().1,
                                "process-consult-native",
                                child_coordinate.clone(),
                                false,
                                selected_store.clone(),
                            );
                        let foreign_parent_batch = foreign_parent_current
                            .plan_child_dispatch(observed_at)
                            .expect("foreign parent roster remains schedulable");
                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &foreign_parent_current,
                            &foreign_parent_batch,
                            &foreign_parent_batch.candidates()[0],
                            None,
                            1,
                            observed_at,
                        )
                        .is_err());

                        let foreign_store = store(
                            "process-consult-native-foreign",
                            "process-consult-native-foreign-incarnation",
                        );
                        let foreign_store_current =
                            consultation_supervision_fixture_for_self_contained_operation_in_store(
                                parent.handle().0,
                                parent.handle().1,
                                "process-consult-native",
                                child_coordinate.clone(),
                                false,
                                foreign_store,
                            );
                        let foreign_store_batch = foreign_store_current
                            .plan_child_dispatch(observed_at)
                            .expect("foreign-store roster remains schedulable");
                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &foreign_store_current,
                            &foreign_store_batch,
                            &foreign_store_batch.candidates()[0],
                            None,
                            1,
                            observed_at,
                        )
                        .is_err());

                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &current,
                            &batch,
                            &candidate,
                            None,
                            0,
                            observed_at,
                        )
                        .is_err());
                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &current,
                            &batch,
                            &candidate,
                            None,
                            1,
                            batch.deadline(),
                        )
                        .is_err());

                        let admitted_generation = parent_start.generation_digest.clone();
                        parent_start.generation_digest =
                            pale_core::records::CompiledGenerationDigest::parse("f".repeat(64))
                                .expect("foreign generation digest");
                        assert!(classify(
                            &driver,
                            custody,
                            anchor,
                            &parent_start,
                            &current,
                            &batch,
                            &candidate,
                            None,
                            1,
                            observed_at,
                        )
                        .is_err());
                        parent_start.generation_digest = admitted_generation;
                    }

                    let classified = classify(
                        &driver,
                        custody,
                        anchor,
                        &parent_start,
                        &current,
                        &batch,
                        &candidate,
                        None,
                        1,
                        observed_at,
                    )
                    .expect("saved-handle recovery admits the exact Consult child");
                    let ResidentProcessOperationRestartDisposition::OperationResolution(recovered) =
                        classified
                    else {
                        panic!("fresh Consult child remains Invoke resolution work")
                    };
                    let child_lineage = recovered.start.lineage.clone();
                    let retained = child_lineage
                        .consult_child()
                        .expect("child classification retains exact roster lineage");
                    assert_eq!(retained.collection_frontier(), batch.collection_frontier());
                    assert_eq!(retained.slot_id(), candidate.slot_id());
                    assert_eq!(retained.child(), candidate.child());

                    if pre_dispatch == ConsultChildPreDispatch::Expire {
                        let expires_at = *recovered
                            .start
                            .control
                            .operation()
                            .operation()
                            .handle()
                            .1
                            .expires_at();
                        let clock = ExactExpiryClock::new(expires_at);
                        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                        let checkpoint_schema =
                            schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
                        let resolver = SelectedStore::available(selected_store.clone());
                        drop(driver);
                        expire_consult_child_before_dispatch(
                            recovered,
                            &mut host,
                            ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody,
                                sessions: &sessions,
                                anchor,
                            },
                            &mut ProvingAuthority,
                            ConsultChildExpiryInputs {
                                clock: &clock,
                                signing_authority: parents.signing,
                                observation_schema: &observation_schema,
                                checkpoint_schema: &checkpoint_schema,
                                checkpoint_authority: &checkpoint_signing,
                                resolver: &resolver,
                            },
                        );
                        return;
                    }

                    if kind == ConsultChildFixtureKind::ProviderBacked {
                        drop(driver);
                        provider_tail(
                            ConsultChildFixture {
                                label,
                                child,
                                parent,
                                generation,
                                parents,
                            },
                            RecoveredConsultChild {
                                parent_start: &parent_start,
                                current: &current,
                                batch: &batch,
                                candidate: &candidate,
                                recovered: recovered.as_ref(),
                                observed_at,
                            },
                            &mut host,
                            custody,
                            anchor,
                        );
                        return;
                    }

                    drop(driver);
                    run_native_consult_child_tail(
                        ConsultChildFixture {
                            label,
                            child,
                            parent,
                            generation,
                            parents,
                        },
                        RecoveredConsultChild {
                            parent_start: &parent_start,
                            current: &current,
                            batch: &batch,
                            candidate: &candidate,
                            recovered: recovered.as_ref(),
                            observed_at,
                        },
                        &mut host,
                        custody,
                        anchor,
                    );
                },
            );
        },
    );
}

#[test]
fn consult_child_native_invoke_recovers_runs_once_and_contributes_only_to_its_slot() {
    run_consult_child_native_invoke(ConsultChildPreDispatch::Continue);
    run_consult_child_native_invoke(ConsultChildPreDispatch::Cancel);
    run_consult_child_native_invoke(ConsultChildPreDispatch::Expire);
}

#[test]
fn native_root_invoke_retains_success_and_handler_failure_without_a_second_call() {
    run_native_case(NativeProcessCase::Success);
    run_native_case(NativeProcessCase::HandlerFailure);
}

#[test]
fn native_root_invoke_returns_the_exact_post_handler_recording_phase() {
    run_native_case(NativeProcessCase::StageCommitFailure);
}

#[test]
fn native_root_invoke_continues_only_the_retained_post_handler_phases() {
    run_native_case(NativeProcessCase::StageSignerContinuation);
    run_native_case(NativeProcessCase::StageResolverContinuation);
    run_native_case(NativeProcessCase::StageCheckpointSignerContinuation);
    run_native_case(NativeProcessCase::StageAttachmentContinuation);
    run_native_case(NativeProcessCase::StageCommitContinuation);
    run_native_case(NativeProcessCase::StageAnchorContinuation);
    run_native_case(NativeProcessCase::StageFinalizationMismatch);
    run_native_case(NativeProcessCase::TerminalPreparationContinuation);
    run_native_case(NativeProcessCase::TerminalCustodyContinuation);
}

#[test]
fn staged_native_cold_restart_records_success_and_failure_without_restaging() {
    run_native_case(NativeProcessCase::StagedRecoverySuccess);
    run_native_case(NativeProcessCase::StagedRecoveryFailure);
    run_native_case(NativeProcessCase::StagedRecoverySuccessAfterCancellation);
    run_native_case(NativeProcessCase::StagedRecoveryFailureAfterCancellation);
}

#[test]
fn native_root_invoke_rejects_foreign_association_before_any_work() {
    run_native_case(NativeProcessCase::CurrentAuthorityRejection);
    run_native_case(NativeProcessCase::AssociationRejection);
}

#[test]
fn native_root_invoke_rejects_unlike_start_evidence_before_owner_work() {
    run_native_case(NativeProcessCase::BadStartEvidence);
}

#[test]
fn native_root_invoke_reclassifies_cancellation_before_dispatch() {
    run_native_case(NativeProcessCase::CancellationBeforeDispatch);
}

#[test]
fn native_root_invoke_preserves_the_unopened_call_at_the_release_boundary() {
    run_native_case(NativeProcessCase::InvocationBoundary);
}

fn native_recovery_input<'a>(
    recovery: OperationControlRecoveryInput<'a>,
) -> ResidentProcessOperationRestartInput<'a> {
    ResidentProcessOperationRestartInput {
        committed_observation_limits: None,
        operation: recovery.operation.context,
        maximum_parent_depth: recovery.operation.maximum_parent_depth,
        current_head: ResidentProcessRestartReadLimits {
            observation: recovery.observation_limits,
            checkpoints: recovery.checkpoint_limits,
            membership: recovery.membership_limits,
        },
        maximum_selection_bytes: 1 << 20,
        a2a_task: None,
        a2a_cancel_task: None,
    }
}

fn native_recovery_association<'a>(
    signing: &'a SigningAuthorityBindingV1,
    input: &ResidentProcessOperationRestartInput<'a>,
) -> ResidentOperationControlAssociation<'a> {
    ResidentOperationControlAssociation {
        current_request_authority: signing,
        historical_handle_authority: input.operation.handle_signing_authority,
        recovery_store: input.operation.store,
        checkpoint_authority: input.operation.checkpoint_authority,
    }
}

#[test]
fn native_cold_restart_retains_a_handler_free_recording_continuation() {
    let cases = Rc::new(Cell::new(0_usize));
    let counted_cases = Rc::clone(&cases);
    with_process_host_native_recovery_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
        let ordinal = counted_cases.replace(counted_cases.get() + 1);
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let application_store = MemoryAuthorityStore::default();
        let mut application = ready_application(&application_store, selected_store.clone(), 81);
        application.fixture_retain_generation(generation);
        let deployment =
            unproved_deployment_for_selector(&selector, selected_store.clone(), application);
        let host = ResidentProcessHost::new(
            unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
            vec![deployment],
            NonZeroUsize::new(1).expect("one deployment"),
            ResidentOperationHost::new(task_state()),
        )
        .expect("native recovery host");
        let restart = native_recovery_input(recovery);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                native_recovery_association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("native restart classification");
        let mut attempt = match disposition {
            ResidentProcessOperationRestartDisposition::Native(attempt) => attempt,
            _ => panic!("native fixture must remain native recovery work"),
        };
        let registry = NativeActionRegistry::new().expect("sealed native registry");
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let resolver = SelectedStore::available(selected_store.clone());
        let checkpoint_input = |suffix: &str| OperationCheckpointInput {
            reference: record_ref(&format!(
                "process-native-recovery/{ordinal}/{suffix}-checkpoint"
            )),
            id: ControlCheckpointIdV1::parse(format!("process-native-recovery-{ordinal}-{suffix}"))
                .expect("recovery checkpoint identity"),
        };

        macro_rules! run {
            ($attempt:expr, $clock:expr, $signer:expr) => {{
                host.recover_native(
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody,
                        sessions,
                        anchor,
                    },
                    &mut ProvingAuthority,
                    ResidentProcessNativeRecoveryInput {
                        attempt: $attempt,
                        current,
                        registry: &registry,
                        contract_set: &contracts,
                        maximum_selection_bytes: 1 << 20,
                        maximum_record_bytes: 1 << 20,
                        recording: ResidentProcessNativeRecoveryRecordingInput {
                            outcome_ref: record_ref(&format!(
                                "process-native-recovery/{ordinal}/outcome"
                            )),
                            outcome_observation_ref: record_ref(&format!(
                                "process-native-recovery/{ordinal}/outcome-observation"
                            )),
                            result_ref: record_ref(&format!(
                                "process-native-recovery/{ordinal}/result"
                            )),
                            diagnostic_ref: record_ref(&format!(
                                "process-native-recovery/{ordinal}/diagnostic"
                            )),
                            terminal_ref: record_ref(&format!(
                                "process-native-recovery/{ordinal}/terminal"
                            )),
                            outcome_checkpoint: checkpoint_input("outcome"),
                            terminal_checkpoint: checkpoint_input("terminal"),
                            clock: $clock,
                            resolve_store: &resolver,
                            checkpoint_schema: &checkpoint_schema,
                            checkpoint_authority,
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: $signer,
                            },
                            maximum_diagnostic_bytes: 65_536,
                            maximum_outcome_bytes: 65_536,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                8, 20_000, 20_000, 8_192, 200_000,
                            )
                            .expect("accumulator limits"),
                        },
                    },
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
            }};
        }

        if ordinal == 1 {
            let foreign = owner_native_signing_authority(selector.principal_id.clone());
            let foreign_clock = IncreasingClock::new(None);
            let result = run!(&attempt, &foreign_clock, &foreign);
            assert!(matches!(
                result,
                Err(ResidentProcessNativeRecoveryFailure::Selection(
                    ResidentProcessHostSelectionError::NativeSigningAuthority
                ))
            ));
            assert_eq!(foreign_clock.reads(), 0);

            let clock = IncreasingClock::new(None);
            let result = run!(&attempt, &clock, signing_authority);
            assert!(matches!(
                result,
                Err(ResidentProcessNativeRecoveryFailure::StagedOutcomeRequired)
            ));
            assert_eq!(clock.reads(), 0);
            attempt.dispatch = ResidentProcessAttemptObservation::CancellationPredecessor;
            let cancellation_clock = IncreasingClock::new(None);
            let result = run!(&attempt, &cancellation_clock, signing_authority);
            assert!(matches!(
                result,
                Err(ResidentProcessNativeRecoveryFailure::StagedOutcomeRequired)
            ));
            assert_eq!(cancellation_clock.reads(), 0);
            return;
        }

        let foreign = owner_native_signing_authority(selector.principal_id.clone());
        let foreign_clock = IncreasingClock::new(None);
        let result = run!(&attempt, &foreign_clock, &foreign);
        assert!(matches!(
            result,
            Err(ResidentProcessNativeRecoveryFailure::Selection(
                ResidentProcessHostSelectionError::NativeSigningAuthority
            ))
        ));
        assert_eq!(foreign_clock.reads(), 0);

        attempt.dispatch = ResidentProcessAttemptObservation::CancellationPredecessor;
        let cancellation_clock = IncreasingClock::new(None);
        let result = run!(&attempt, &cancellation_clock, signing_authority);
        assert!(matches!(
            result,
            Err(ResidentProcessNativeRecoveryFailure::CancellationRequested)
        ));
        assert_eq!(cancellation_clock.reads(), 0);

        attempt.dispatch = ResidentProcessAttemptObservation::CurrentHead;
        let recovered_dispatched = attempt.start.control.observation().selected();
        assert!(matches!(
            recovered_dispatched.1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { .. }
            }
        ));
        let recovered_dispatched_ref = recovered_dispatched.0.clone();
        let recovered_dispatched_digest = recovered_dispatched
            .1
            .canonical_operation_observation_sha256()
            .clone();
        let operation_handle_key =
            crate::resident::operation_observation_custody::operation_handle_key(
                attempt
                    .start
                    .control
                    .operation()
                    .operation()
                    .handle()
                    .1
                    .operation_handle_id(),
            )
            .expect("recovered operation handle key");
        let recovery_floor = current
            .now
            .max(
                *attempt
                    .start
                    .control
                    .observation()
                    .selected()
                    .1
                    .observed_at(),
            )
            .max(
                *attempt
                    .start
                    .control
                    .observation()
                    .current_checkpoint()
                    .1
                    .issued_at(),
            );
        let clock = IncreasingClock::starting_at(recovery_floor.get(), Some(4));
        let result = run!(&attempt, &clock, signing_authority);
        let Err(ResidentProcessNativeRecoveryFailure::Recording(
            NativeOperationRecordingFailure::Unstaged { outcome, .. },
        )) = result
        else {
            panic!("recomputed outcome must retain its first recording phase")
        };
        assert!(matches!(
            outcome.outcome(),
            crate::resident::operation_dispatch::NativeOperationOutcome::Succeeded(_)
        ));
        assert_eq!(clock.reads(), 4);

        let recovered_checkpoint = attempt.start.control.observation().current_checkpoint();
        let recording_clock = IncreasingClock::starting_at(recovery_floor.get() + 4, None);
        let continued_outcome_ref = record_ref(&format!(
            "process-native-recovery/{ordinal}/continued-outcome"
        ));
        let continued_outcome_observation_ref = record_ref(&format!(
            "process-native-recovery/{ordinal}/continued-outcome-observation"
        ));
        let result_signs = Cell::new(0_usize);
        custody::with_service_custody_and_handle(
            recovered_checkpoint.1,
            None,
            None,
            2,
            2,
            false,
            &[],
            None,
            Rc::new(RefCell::new(None)),
            |continuation_custody, continuation_anchor| {
                let result = record_native_operation_outcome(
                    *outcome,
                    NativeOperationOutcomeRecordingInput {
                        checkpoint: recovered_checkpoint,
                        outcome_ref: continued_outcome_ref,
                        outcome_observation_ref: continued_outcome_observation_ref.clone(),
                        result_ref: record_ref(&format!(
                            "process-native-recovery/{ordinal}/continued-result"
                        )),
                        diagnostic_ref: record_ref(&format!(
                            "process-native-recovery/{ordinal}/continued-diagnostic"
                        )),
                        terminal_ref: record_ref(&format!(
                            "process-native-recovery/{ordinal}/continued-terminal"
                        )),
                        outcome_checkpoint: OperationCheckpointInput {
                            reference: record_ref(&format!(
                                "process-native-recovery/{ordinal}/continued-outcome-checkpoint"
                            )),
                            id: ControlCheckpointIdV1::parse(format!(
                                "process-native-recovery-{ordinal}-continued-outcome"
                            ))
                            .expect("continued outcome checkpoint identity"),
                        },
                        terminal_checkpoint: OperationCheckpointInput {
                            reference: record_ref(&format!(
                                "process-native-recovery/{ordinal}/unused-terminal-checkpoint"
                            )),
                            id: ControlCheckpointIdV1::parse(format!(
                                "process-native-recovery-{ordinal}-unused-terminal"
                            ))
                            .expect("unused terminal checkpoint identity"),
                        },
                        clock: &recording_clock,
                        resolve_store: &resolver,
                        checkpoint_schema: &checkpoint_schema,
                        checkpoint_authority,
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority,
                        },
                        maximum_diagnostic_bytes: 65_536,
                        maximum_outcome_bytes: 65_536,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            8, 20_000, 20_000, 8_192, 200_000,
                        )
                        .expect("continued outcome accumulator limits"),
                    },
                    continuation_custody,
                    continuation_anchor,
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| {
                        result_signs.set(result_signs.get() + 1);
                        Err(modeled_signing_failure())
                    },
                );
                let Err(NativeOperationRecordingFailure::TerminalPreparation { staged, failure }) =
                    result
                else {
                    panic!("terminal signer refusal must retain the exact staged outcome")
                };
                assert!(matches!(
                    *failure,
                    OperationAppendFailure::Operation(OperationAttemptError::Admission(_))
                ));
                assert_eq!(result_signs.get(), 1);
                let (staged_checkpoint_ref, staged_checkpoint) = staged.checkpoint();
                let staged_checkpoint_ref = staged_checkpoint_ref.clone();
                let completed_at = *staged_checkpoint.issued_at();
                let staged_sequence = staged_checkpoint.domain_sequence().get();
                assert_eq!(
                    staged_checkpoint.previous_control_checkpoint_ref().as_ref(),
                    Some(recovered_checkpoint.0)
                );
                assert_eq!(
                    staged_checkpoint
                        .previous_control_checkpoint_sha256()
                        .as_ref(),
                    Some(recovered_checkpoint.1.canonical_control_checkpoint_sha256())
                );

                let staged_observation = pale_ports::OperationCustodyPort::read_index(
                    continuation_custody,
                    &selected_store,
                    &OperationCustodyIndexQuery::LatestObservation(operation_handle_key.clone()),
                    1 << 20,
                )
                .expect("staged observation index read")
                .into_record()
                .expect("staged observation remains durable");
                assert_eq!(
                    staged_observation.coordinate().record_ref(),
                    &continued_outcome_observation_ref
                );
                let staged_observation_wire: OperationObservationWireV1 =
                    serde_json::from_slice(staged_observation.canonical_record_jcs().as_slice())
                        .expect("staged observation wire");
                assert_eq!(
                    staged_observation_wire.predecessor_ref().as_ref(),
                    Some(&recovered_dispatched_ref)
                );
                assert_eq!(
                    staged_observation_wire.predecessor_sha256().as_ref(),
                    Some(&recovered_dispatched_digest)
                );

                let staged_checkpoint_record = pale_ports::OperationCustodyPort::read_index(
                    continuation_custody,
                    &selected_store,
                    &OperationCustodyIndexQuery::ObservationCheckpoint(
                        OperationCustodyIndexKey::for_observation(staged_observation.coordinate()),
                    ),
                    1 << 20,
                )
                .expect("staged checkpoint index read")
                .into_record()
                .expect("staged checkpoint remains durable");
                assert_eq!(
                    staged_checkpoint_record.coordinate().record_ref(),
                    &staged_checkpoint_ref
                );
                let staged_checkpoint_wire: ControlStateCheckpointWireV1 = serde_json::from_slice(
                    staged_checkpoint_record.canonical_record_jcs().as_slice(),
                )
                .expect("staged checkpoint wire");
                let durable_staged_checkpoint =
                    pale_core::records::ControlStateCheckpointV1::admit_successor_wire(
                        staged_checkpoint_wire,
                        recovered_checkpoint.0,
                        recovered_checkpoint.1,
                        &checkpoint_schema,
                        checkpoint_authority,
                        |_, _, _, actual| {
                            assert_eq!(actual, &signature());
                            Ok(())
                        },
                    )
                    .expect("durable staged checkpoint admits as the exact successor");
                assert_eq!(
                    durable_staged_checkpoint
                        .previous_control_checkpoint_ref()
                        .as_ref(),
                    Some(recovered_checkpoint.0)
                );
                assert_eq!(
                    durable_staged_checkpoint
                        .previous_control_checkpoint_sha256()
                        .as_ref(),
                    Some(recovered_checkpoint.1.canonical_control_checkpoint_sha256())
                );
                assert_eq!(
                    staged_checkpoint_record.canonical_record_jcs().as_slice(),
                    pale_core::canon::to_canonical_string(staged_checkpoint)
                        .expect("staged checkpoint canonical encoding")
                        .as_bytes()
                );

                let terminal_ref = record_ref(&format!(
                    "process-native-recovery/{ordinal}/continued-terminal-after-signing"
                ));
                let terminal_checkpoint_ref = record_ref(&format!(
                    "process-native-recovery/{ordinal}/continued-terminal-checkpoint"
                ));
                let terminal_diagnostic_ref = record_ref(&format!(
                    "process-native-recovery/{ordinal}/continued-diagnostic-after-signing"
                ));
                let terminal_clock = IncreasingClock::starting_at(completed_at.get() + 1, None);
                let completed = staged
                    .record_terminal(
                        NativeOperationTerminalRecordingInput {
                            result_ref: record_ref(&format!(
                                "process-native-recovery/{ordinal}/continued-result-after-signing"
                            )),
                            diagnostic_ref: terminal_diagnostic_ref,
                            terminal_ref: terminal_ref.clone(),
                            terminal_checkpoint: OperationCheckpointInput {
                                reference: terminal_checkpoint_ref.clone(),
                                id: ControlCheckpointIdV1::parse(format!(
                                    "process-native-recovery-{ordinal}-continued-terminal"
                                ))
                                .expect("continued terminal checkpoint identity"),
                            },
                            clock: &terminal_clock,
                            resolve_store: &resolver,
                            checkpoint_schema: &checkpoint_schema,
                            checkpoint_authority,
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority,
                            },
                            maximum_diagnostic_bytes: 65_536,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                8, 20_000, 20_000, 8_192, 200_000,
                            )
                            .expect("continued terminal accumulator limits"),
                        },
                        continuation_custody,
                        continuation_anchor,
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                        |_, _, _| Ok(signature()),
                        |_, _, _| {
                            result_signs.set(result_signs.get() + 1);
                            Ok(signature())
                        },
                    )
                    .expect("the retained staged outcome reaches terminal custody");

                let result = completed
                    .result()
                    .expect("the recovered native action succeeds");
                assert_eq!(result.completed_at(), &completed_at);
                assert_eq!(result_signs.get(), 2);
                let (actual_terminal_ref, terminal) = completed.observation();
                assert_eq!(actual_terminal_ref, &terminal_ref);
                assert_eq!(
                    terminal.predecessor_ref().as_ref(),
                    Some(&continued_outcome_observation_ref)
                );
                assert!(terminal.state().is_terminal());
                let terminal_custody = completed.custody().candidate();
                assert_eq!(
                    terminal_custody
                        .checkpoint()
                        .predecessor_record()
                        .expect("terminal custody retains the staged checkpoint")
                        .coordinate()
                        .record_ref(),
                    &staged_checkpoint_ref
                );
                assert_eq!(
                    terminal_custody
                        .checkpoint()
                        .successor_record()
                        .coordinate()
                        .record_ref(),
                    &terminal_checkpoint_ref
                );
                assert_eq!(
                    terminal_custody
                        .checkpoint()
                        .successor()
                        .domain_sequence()
                        .get(),
                    staged_sequence + 1
                );

                let terminal_record = terminal_custody
                    .record(OperationCustodyRecordKind::OperationObservation)
                    .expect("terminal custody retains the terminal observation");
                let terminal_coordinate = terminal_record.coordinate().clone();
                let terminal_bytes = terminal_record.canonical_record_jcs().as_slice().to_vec();
                let read_back = pale_ports::OperationCustodyPort::read_operation_record(
                    continuation_custody,
                    &selected_store,
                    &terminal_coordinate,
                    1 << 20,
                )
                .expect("terminal read-back is available")
                .expect("terminal observation is durable");
                assert_eq!(read_back.coordinate(), &terminal_coordinate);
                assert_eq!(
                    read_back.canonical_record_jcs().as_slice(),
                    terminal_bytes.as_slice()
                );
            },
        );
        assert_eq!(
            clock.reads(),
            4,
            "recording the retained outcome must not read the recovery clock again"
        );
    });
    assert_eq!(cases.get(), 2);
}
