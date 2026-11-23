//! Retained root-Invoke recovery through the public service composition.

use super::*;
use crate::actions::NativeActionRegistry;
use crate::local_operation_start_fixture::{
    record_ref, schema_digest, signature, store_coordinate,
};
use crate::resident::event_custody::local_operation_projection::tests::{
    producer, writer, PublicationStore,
};
use crate::resident::event_custody::{
    EventPublicationRaceClosure, EventPublicationRaceRejection, EventPublicationRaceVerifier,
    EventPublicationState, LocalOperationPublicationCurrent, LocalOperationPublicationIssuance,
    LocalOperationPublicationReadmission,
};
use crate::resident::journal::tests::MemoryAuthorityStore;
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_native_recovery_fixtures, with_process_host_prepared_restart_fixtures,
    with_process_host_provider_recovery_fixtures,
    with_process_host_provider_resolved_restart_fixture, with_process_host_reconciliation_fixtures,
    ProcessHostRestartFixture,
};
use crate::resident::operation_observation_custody::tests::local_start::{
    initial_anchor, source_generation,
};
use crate::resident::process_host::tests::dispatch_prepared_recovery::{
    accumulator_limits as prepared_accumulator_limits, provider_admission,
    ProviderAdmissionFixture, RecoveryReleaseClock, UnusedRecoveredProviderEvidence,
};
use crate::resident::process_host::tests::provider_recovery::{
    association, host_with_generation, restart_input, AppendAnchor, AppendCustody, IncreasingClock,
    ProviderRecoveryInvocation, SelectedStore,
};
use crate::resident::process_host::tests::publication::{
    a2a_recovery_limits, publication_contexts, publication_schemas, publication_source,
    recovery_limits, PublicationSources,
};
use crate::resident::process_host::tests::{
    integrated_signing_authority, selector_from_signing_authority, unproved_selector,
};
use crate::resident::{
    A2aTaskPublicationRecoveryRequest, A2aTaskPublicationSourceRecoveryError,
    LocalOperationPublicationRecoveryRequest, LocalOperationPublicationSourceRecoveryError,
    LocalOperationPublicationSweepLimits, OperationCheckpointInput, ProviderOperationInvokeInput,
    RecoveredA2aTaskPublicationSource, RecoveredOperationControl,
    ResidentServiceDeploymentSelector, ResidentServiceInvokeBackgroundRecoveryInputs,
    ResidentServiceInvokeRecoveryInputs, ResidentServiceNativeRecoveryInputs,
    ResidentServiceOperationResolutionInputs, ResidentServicePreDispatchExpiryContinuationInputs,
    ResidentServicePreDispatchExpiryPreparationInputs, ResidentServicePublicationInputPort,
    ResidentServicePublicationInputs,
};
use crate::session_admission_fixture::ProvingAuthority;
use pale_core::records::{
    BoundedResidentTextV1, CanonicalSignatureV1, CapabilityAttemptIdV1, CommonRequestEnvelopeV1,
    ContractSetV1, ControlCheckpointIdV1, ControlCheckpointSigningAuthorityAdmissionContextV1,
    EventReferencePreimageV1, GenealogyEdgeDigest, GenealogyEdgePreimageV1,
    InteractionOccurrenceDigest, InteractionOccurrencePreimageV1, InvalidResidentRecordV1,
    PrincipalResidentWriterFenceV1, ProducerIncarnationV1, ReconciliationFrontierDigest,
    ReconciliationFrontierPreimageV1, RecordSchemaIdentity, ResidentPrincipalIdV1,
    ResidentRecordRefV1, SchemaDigest, SignedResidentAdmissionContextV1, SigningAuthorityBindingV1,
    UnixTimeV1, ADAPTER_FAILURE_V1_SCHEMA_ID, CAPABILITY_RESULT_V1_SCHEMA_ID,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorPreparationLimits, ControlRecord, ControlRecordCoordinate, ControlStoreErrorKind,
    CryptographicRandomSourceError, CryptographicRandomSourcePort, OperationCustodyReadBackOutcome,
    ResidentOperationFrameFailure, ResidentOperationFrameLocator, WallClockSourceError,
    WallClockSourcePort,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

fn reborrow_recovery_admission<'short>(
    admission: &'short mut crate::resident::OperationControlRecoveryAdmission<'_>,
) -> crate::resident::OperationControlRecoveryAdmission<'short> {
    crate::resident::OperationControlRecoveryAdmission {
        admit_request: &mut *admission.admit_request,
        admit_handle_authority: &mut *admission.admit_handle_authority,
        verify_handle_signature: &mut *admission.verify_handle_signature,
        admit_checkpoint: &mut *admission.admit_checkpoint,
        resolve_observation_signer: &mut *admission.resolve_observation_signer,
        admit_observation_state: &mut *admission.admit_observation_state,
        admit_rotation: &mut *admission.admit_rotation,
        verify_observation_signature: &mut *admission.verify_observation_signature,
        verify_checkpoint_signature: &mut *admission.verify_checkpoint_signature,
    }
}

fn repeated_restart_inputs(
    recovery: crate::resident::OperationControlRecoveryInput<'_>,
) -> [super::super::restart::ResidentProcessOperationRestartInput<'_>; 3] {
    let crate::resident::OperationControlRecoveryInput {
        operation,
        observation,
        observation_limits,
        checkpoint_limits,
        membership_limits,
    } = recovery;
    let crate::resident::LocalOperationHandleRecoveryInput {
        context,
        expected_handle,
        maximum_parent_depth,
    } = operation;
    let make = || {
        restart_input(crate::resident::OperationControlRecoveryInput {
            operation: crate::resident::LocalOperationHandleRecoveryInput {
                context,
                expected_handle,
                maximum_parent_depth,
            },
            observation,
            observation_limits,
            checkpoint_limits,
            membership_limits,
        })
    };
    [make(), make(), make()]
}

type TestOccurrenceSigner = fn(
    &[u8],
    &InteractionOccurrencePreimageV1,
    &InteractionOccurrenceDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>;
type TestEdgeSigner = fn(
    &[u8],
    &GenealogyEdgePreimageV1,
    &GenealogyEdgeDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>;
type TestFrontierSigner = fn(
    &[u8],
    &ReconciliationFrontierPreimageV1,
    &ReconciliationFrontierDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>;
type TestEventSourceAdmission =
    fn(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>;
type TestAuthorityResolver =
    fn(&ControlRecord) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>;
type TestOccurrenceVerifier = fn(
    &[u8],
    &InteractionOccurrencePreimageV1,
    &InteractionOccurrenceDigest,
    &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1>;
type TestEdgeVerifier = fn(
    &[u8],
    &GenealogyEdgePreimageV1,
    &GenealogyEdgeDigest,
    &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1>;
type TestFrontierVerifier = fn(
    &[u8],
    &ReconciliationFrontierPreimageV1,
    &ReconciliationFrontierDigest,
    &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1>;
type TestLocalPublicationRecovery = for<'request> fn(
    &LocalOperationPublicationRecoveryRequest<'request>,
) -> Result<
    RecoveredOperationControl,
    LocalOperationPublicationSourceRecoveryError,
>;
type TestA2aPublicationRecovery = for<'request> fn(
    &A2aTaskPublicationRecoveryRequest<'request>,
) -> Result<
    RecoveredA2aTaskPublicationSource,
    A2aTaskPublicationSourceRecoveryError,
>;

struct AcceptPublicationRace;

impl EventPublicationRaceVerifier for AcceptPublicationRace {
    fn verify(
        &self,
        _: &EventPublicationRaceClosure<'_>,
    ) -> Result<(), EventPublicationRaceRejection> {
        Ok(())
    }
}

fn unexpected_occurrence_signing(
    _: &[u8],
    _: &InteractionOccurrencePreimageV1,
    _: &InteractionOccurrenceDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
    panic!("unavailable source recovery must precede occurrence signing")
}

fn unexpected_edge_signing(
    _: &[u8],
    _: &GenealogyEdgePreimageV1,
    _: &GenealogyEdgeDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
    panic!("unavailable source recovery must precede edge signing")
}

fn unexpected_frontier_signing(
    _: &[u8],
    _: &ReconciliationFrontierPreimageV1,
    _: &ReconciliationFrontierDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
    panic!("unavailable source recovery must precede frontier signing")
}

fn unexpected_event_source_admission(
    _: &EventReferencePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("unavailable source recovery must precede Event readmission")
}

fn unexpected_publication_authority_resolution(
    _: &ControlRecord,
) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1> {
    panic!("unavailable source recovery must precede authority resolution")
}

fn unexpected_occurrence_verification(
    _: &[u8],
    _: &InteractionOccurrencePreimageV1,
    _: &InteractionOccurrenceDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("unavailable source recovery must precede occurrence verification")
}

fn unexpected_edge_verification(
    _: &[u8],
    _: &GenealogyEdgePreimageV1,
    _: &GenealogyEdgeDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("unavailable source recovery must precede edge verification")
}

fn unexpected_frontier_verification(
    _: &[u8],
    _: &ReconciliationFrontierPreimageV1,
    _: &ReconciliationFrontierDigest,
    _: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("unavailable source recovery must precede frontier verification")
}

fn unavailable_local_publication_recovery(
    _: &LocalOperationPublicationRecoveryRequest<'_>,
) -> Result<RecoveredOperationControl, LocalOperationPublicationSourceRecoveryError> {
    Err(LocalOperationPublicationSourceRecoveryError::Unavailable)
}

fn unexpected_a2a_publication_recovery(
    _: &A2aTaskPublicationRecoveryRequest<'_>,
) -> Result<RecoveredA2aTaskPublicationSource, A2aTaskPublicationSourceRecoveryError> {
    panic!("the retained local source must not select A2A recovery")
}

struct UnavailableLocalPublicationInputPort<'authority> {
    expected_deployment: ResidentServiceDeploymentSelector,
    event_custody: PublicationStore<'authority>,
    producer_allocation_ref: ResidentRecordRefV1,
    producer: ProducerIncarnationV1,
    writer_fence_ref: ResidentRecordRefV1,
    writer_fence: PrincipalResidentWriterFenceV1,
    race_verifier: AcceptPublicationRace,
    publication_authority: SigningAuthorityBindingV1,
    schemas: (
        RecordSchemaIdentity,
        RecordSchemaIdentity,
        RecordSchemaIdentity,
    ),
    sign_occurrence: TestOccurrenceSigner,
    sign_edge: TestEdgeSigner,
    sign_frontier: TestFrontierSigner,
    admit_event_source: TestEventSourceAdmission,
    resolve_occurrence_authority: TestAuthorityResolver,
    verify_occurrence_signature: TestOccurrenceVerifier,
    resolve_edge_authority: TestAuthorityResolver,
    verify_edge_signature: TestEdgeVerifier,
    resolve_frontier_authority: TestAuthorityResolver,
    verify_frontier_signature: TestFrontierVerifier,
    recover_local: TestLocalPublicationRecovery,
    recover_a2a: TestA2aPublicationRecovery,
    lends: Rc<Cell<usize>>,
}

impl<'authority> UnavailableLocalPublicationInputPort<'authority> {
    fn new(
        authority: &'authority MemoryAuthorityStore,
        expected_deployment: ResidentServiceDeploymentSelector,
        publication_authority: SigningAuthorityBindingV1,
        lends: Rc<Cell<usize>>,
    ) -> Self {
        Self {
            expected_deployment,
            event_custody: PublicationStore::new(authority),
            producer_allocation_ref: record_ref("publication/producer"),
            producer: producer(),
            writer_fence_ref: record_ref("publication/writer"),
            writer_fence: writer(),
            race_verifier: AcceptPublicationRace,
            publication_authority,
            schemas: publication_schemas(),
            sign_occurrence: unexpected_occurrence_signing,
            sign_edge: unexpected_edge_signing,
            sign_frontier: unexpected_frontier_signing,
            admit_event_source: unexpected_event_source_admission,
            resolve_occurrence_authority: unexpected_publication_authority_resolution,
            verify_occurrence_signature: unexpected_occurrence_verification,
            resolve_edge_authority: unexpected_publication_authority_resolution,
            verify_edge_signature: unexpected_edge_verification,
            resolve_frontier_authority: unexpected_publication_authority_resolution,
            verify_frontier_signature: unexpected_frontier_verification,
            recover_local: unavailable_local_publication_recovery,
            recover_a2a: unexpected_a2a_publication_recovery,
            lends,
        }
    }
}

impl ResidentServicePublicationInputPort for UnavailableLocalPublicationInputPort<'_> {
    fn publication_inputs<'a>(
        &'a mut self,
        deployment: &ResidentServiceDeploymentSelector,
    ) -> Result<ResidentServicePublicationInputs<'a>, ResidentOperationFrameFailure> {
        assert_eq!(deployment, &self.expected_deployment);
        self.lends.set(self.lends.get() + 1);
        Ok(ResidentServicePublicationInputs {
            event_custody: &mut self.event_custody,
            sweep_limits: LocalOperationPublicationSweepLimits::new(1, 1, 65_536, 65_536)
                .expect("positive publication limits"),
            local_recovery_limits: recovery_limits(),
            a2a_recovery_limits: a2a_recovery_limits(),
            current: LocalOperationPublicationCurrent {
                producer_allocation_ref: &self.producer_allocation_ref,
                producer: &self.producer,
                writer_fence_ref: &self.writer_fence_ref,
                writer_fence: &self.writer_fence,
                publication_state: EventPublicationState::Active,
                race_verifier: &self.race_verifier,
                signing_contexts: publication_contexts(
                    &self.publication_authority,
                    &self.schemas.0,
                    &self.schemas.1,
                    &self.schemas.2,
                ),
            },
            issuance: LocalOperationPublicationIssuance {
                sign_occurrence: &mut self.sign_occurrence,
                sign_edge: &mut self.sign_edge,
                sign_frontier: &mut self.sign_frontier,
            },
            readmission: LocalOperationPublicationReadmission {
                admit_event_source: &mut self.admit_event_source,
                resolve_occurrence_authority: &mut self.resolve_occurrence_authority,
                verify_occurrence_signature: &mut self.verify_occurrence_signature,
                resolve_edge_authority: &mut self.resolve_edge_authority,
                verify_edge_signature: &mut self.verify_edge_signature,
                resolve_frontier_authority: &mut self.resolve_frontier_authority,
                verify_frontier_signature: &mut self.verify_frontier_signature,
            },
            recover_local: &mut self.recover_local,
            recover_a2a: &mut self.recover_a2a,
        })
    }
}

#[test]
fn complete_service_publication_lender_keeps_the_retained_cursor_after_recovery_failure() {
    let authority = MemoryAuthorityStore::default();
    let selected_store = store_coordinate();
    let selector = unproved_selector(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal identity"),
        selected_store.clone(),
    );
    let publication_authority = integrated_signing_authority(&selector);
    let (generation, _) = source_generation();
    let mut host = host_with_generation(&authority, &selector, &selected_store, generation);
    let mut sources = PublicationSources::one(selected_store.clone(), publication_source());
    let lends = Rc::new(Cell::new(0));
    let publication_inputs = UnavailableLocalPublicationInputPort::new(
        &authority,
        service_deployment_selector(&selector),
        publication_authority,
        Rc::clone(&lends),
    );
    let mut service = InvokeRecoveryAdapter::new(
        ProviderRecoveryInputs {
            recording: None,
            lends: Rc::new(AtomicUsize::new(0)),
        },
        Box::new(publication_inputs),
        UnavailableResidentServiceDelegateInputs,
        (),
        (),
        1,
    );
    let mut anchor = initial_anchor();

    for _ in 0..2 {
        let result = program::ResidentProgramRootRecoveryInputPort::recover_publication(
            &mut service,
            &mut host,
            &selector,
            super::super::ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut sources,
                sessions: &authority,
                anchor: &mut anchor,
            },
        );
        assert!(matches!(
            result,
            Err(program::ResidentProgramTurnError::RecoveryUnavailable)
        ));
        assert!(host.deployments[0].application.journal_head().is_none());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    }

    assert_eq!(lends.get(), 2, "each service turn borrows physical inputs");
    assert_eq!(
        sources.reads(),
        1,
        "the second turn resumes the retained source instead of scanning it again"
    );
}

struct FailOnceClock {
    next: AtomicU64,
    reads: AtomicUsize,
    fail_at: usize,
}

#[test]
fn recovery_only_sources_leave_capacity_for_later_frozen_prefix_turns() {
    with_process_host_reconciliation_fixtures(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            mut admission,
            signing_authority,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        custody.repeat_start_source_once();
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let [first_restart, second_restart, exhausted_restart] = repeated_restart_inputs(recovery);
        let mut retained = InvokeRecoveryAdapter::new(
            (),
            Box::new(UnavailableResidentServicePublicationInputs),
            UnavailableResidentServiceDelegateInputs,
            (),
            (),
            1,
        );

        for (ordinal, restart) in [first_restart, second_restart].into_iter().enumerate() {
            let turn = host
                .recover_one_root_provider_start(
                    &selector,
                    super::super::ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut *custody,
                        sessions,
                        anchor: &mut *anchor,
                    },
                    restart,
                    reborrow_recovery_admission(&mut admission),
                    1 << 20,
                    &mut retained,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "recovery-only source {} is reported: {error:?}",
                        ordinal + 1
                    )
                });
            assert!(matches!(
                turn,
                super::super::operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::ReportedRecoveryOnly(_)
            ));
            assert_eq!(
                retained.retained.iter().count(),
                0,
                "a closed recovery-only report consumes no retained capacity"
            );
        }

        let exhausted = host
            .recover_one_root_provider_start(
                &selector,
                super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                exhausted_restart,
                reborrow_recovery_admission(&mut admission),
                1 << 20,
                &mut retained,
            )
            .expect("the later turn observes the exhausted frozen prefix");
        assert!(matches!(
            exhausted,
            super::super::operation_recovery_sweep::ResidentProcessOperationRecoveryTurn::Exhausted
        ));
        assert_eq!(custody.start_reads(), 2);
        assert_eq!(retained.retained.iter().count(), 0);
    });
}

impl FailOnceClock {
    fn new(first: u64, fail_at: usize) -> Self {
        Self {
            next: AtomicU64::new(first),
            reads: AtomicUsize::new(0),
            fail_at,
        }
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl WallClockSourcePort for FailOnceClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let read = self.reads.fetch_add(1, Ordering::SeqCst) + 1;
        let value = self.next.fetch_add(1, Ordering::SeqCst);
        if read == self.fail_at {
            return Err(WallClockSourceError::from_source(std::io::Error::other(
                "modeled post-handler recording interruption",
            )));
        }
        UnixTimeV1::new(value).map_err(WallClockSourceError::from_source)
    }
}

/// Test clock that always returns one exact expiry boundary.
pub(in crate::resident::process_host) struct ExactExpiryClock {
    now: UnixTimeV1,
    reads: AtomicUsize,
}

impl ExactExpiryClock {
    /// Create a clock fixed at `now`.
    pub(in crate::resident::process_host) fn new(now: UnixTimeV1) -> Self {
        Self {
            now,
            reads: AtomicUsize::new(0),
        }
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl WallClockSourcePort for ExactExpiryClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.now)
    }
}

#[allow(clippy::too_many_arguments)]
fn lend_not_expired_pre_dispatch_inputs<Output, UseInputs>(
    now: UnixTimeV1,
    signing_authority: &SigningAuthorityBindingV1,
    observation_schema: &SchemaDigest,
    checkpoint_schema: &SchemaDigest,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    resolver: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
    use_inputs: UseInputs,
) -> Output
where
    UseInputs:
        for<'input> FnOnce(ResidentServicePreDispatchExpiryPreparationInputs<'input>) -> Output,
{
    let clock = ExactExpiryClock::new(now);
    let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
    let mut sign_observation = |_: &[u8], _: &_, _: &_| Ok(signature());
    let mut sign_checkpoint = |_: &[u8], _: &_, _: &_| Ok(signature());
    use_inputs(ResidentServicePreDispatchExpiryPreparationInputs {
        clock: &clock,
        diagnostic_ref: record_ref("retained/not-expired-diagnostic"),
        observation_ref: record_ref("retained/not-expired-observation"),
        signing: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: observation_schema,
            signing_authority,
        },
        checkpoint: OperationCheckpointInput {
            reference: record_ref("retained/not-expired-checkpoint"),
            id: ControlCheckpointIdV1::parse("retained-not-expired-checkpoint")
                .expect("bounded checkpoint identity"),
        },
        checkpoint_schema,
        checkpoint_authority,
        resolve_store: resolver,
        maximum_diagnostic_bytes: 65_536,
        maximum_record_bytes: 1 << 20,
        accumulator_limits: prepared_accumulator_limits(),
        admit_rotation: &mut admit_rotation,
        sign_observation: &mut sign_observation,
        sign_checkpoint: &mut sign_checkpoint,
    })
}

struct NativeRecoveryInputs<'a> {
    current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    registry: &'a NativeActionRegistry,
    contracts: &'a ContractSetV1,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    observation_schema: &'a SchemaDigest,
    checkpoint_schema: &'a SchemaDigest,
    clock: &'a FailOnceClock,
    resolver: &'a SelectedStore,
    lends: Rc<AtomicUsize>,
}

struct ProviderRecoveryInputs<'a> {
    recording: Option<ProviderOperationInvokeInput<'a>>,
    lends: Rc<AtomicUsize>,
}

struct RetainedRandom {
    calls: AtomicUsize,
}

impl RetainedRandom {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl CryptographicRandomSourcePort for RetainedRandom {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        destination.fill(u8::try_from(call).expect("bounded provider-attempt draw"));
        Ok(())
    }
}

struct RetainedProviderResolvedInputs<'a> {
    session: &'a crate::resident::AdmittedAmbientSession,
    current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    kind: &'a BoundedResidentTextV1,
    version: &'a BoundedResidentTextV1,
    result_schema: &'a SchemaDigest,
    failure_schema: &'a SchemaDigest,
    contracts: &'a ContractSetV1,
    signing_authority: &'a SigningAuthorityBindingV1,
    observation_schema: &'a SchemaDigest,
    checkpoint_schema: &'a SchemaDigest,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    resolver: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    random: &'a dyn CryptographicRandomSourcePort,
    observation_signs: Rc<Cell<usize>>,
    checkpoint_signs: Rc<Cell<usize>>,
    lends: Rc<Cell<usize>>,
}

impl ResidentServiceInvokeInputPort for RetainedProviderResolvedInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<&ControlRecordCoordinate>,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_pre_dispatch_expiry_preparation_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServicePreDispatchExpiryPreparationInputs<'input>) -> Output,
    {
        Ok(lend_not_expired_pre_dispatch_inputs(
            self.current.now,
            self.signing_authority,
            self.observation_schema,
            self.checkpoint_schema,
            self.checkpoint_authority,
            self.resolver,
            use_inputs,
        ))
    }

    fn with_provider_resolved_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderResolvedInputs<'input>) -> Output,
    {
        self.lends.set(self.lends.get() + 1);
        let observation_signs = Rc::clone(&self.observation_signs);
        let checkpoint_signs = Rc::clone(&self.checkpoint_signs);
        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
        let mut sign_observation = move |_: &[u8], _: &_, _: &_| {
            observation_signs.set(observation_signs.get() + 1);
            Ok(signature())
        };
        let mut sign_checkpoint = move |_: &[u8], _: &_, _: &_| {
            checkpoint_signs.set(checkpoint_signs.get() + 1);
            Ok(signature())
        };
        Ok(use_inputs(ResidentServiceProviderResolvedInputs {
            session: self.session,
            current: self.current,
            implementation_kind: self.kind,
            implementation_version: self.version,
            result_schema: self.result_schema,
            result_authority: self.signing_authority,
            contract_set: self.contracts,
            failure_schema: self.failure_schema,
            failure_authority: self.signing_authority,
            safe_failure_proof:
                crate::provider_invocation::ProviderInvocationSafeFailureProof::BeforeDispatch,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: self.observation_schema,
                signing_authority: self.signing_authority,
            },
            checkpoint_schema: self.checkpoint_schema,
            checkpoint_authority: self.checkpoint_authority,
            resolve_store: self.resolver,
            accumulator_limits: AccumulatorPreparationLimits::new(
                8, 20_000, 20_000, 8_192, 200_000,
            )
            .expect("retained provider-resolution accumulator limits"),
            maximum_selection_bytes: 1 << 20,
            maximum_record_bytes: 1 << 20,
            random: self.random,
            admit_rotation: &mut admit_rotation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
        }))
    }
}

struct RetainedNativePreparedInputs<'a> {
    current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    signing_authority: &'a SigningAuthorityBindingV1,
    observation_schema: &'a SchemaDigest,
    checkpoint_schema: &'a SchemaDigest,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    resolver: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    registry: &'a NativeActionRegistry,
    contracts: &'a ContractSetV1,
    observation_signs: Rc<Cell<usize>>,
    checkpoint_signs: Rc<Cell<usize>>,
    lends: Rc<Cell<usize>>,
}

impl ResidentServiceInvokeInputPort for RetainedNativePreparedInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<&ControlRecordCoordinate>,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_pre_dispatch_expiry_preparation_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServicePreDispatchExpiryPreparationInputs<'input>) -> Output,
    {
        Ok(lend_not_expired_pre_dispatch_inputs(
            self.current.now,
            self.signing_authority,
            self.observation_schema,
            self.checkpoint_schema,
            self.checkpoint_authority,
            self.resolver,
            use_inputs,
        ))
    }

    fn with_native_prepared_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceNativePreparedInputs<'input>) -> Output,
    {
        self.lends.set(self.lends.get() + 1);
        let observation_signs = Rc::clone(&self.observation_signs);
        let checkpoint_signs = Rc::clone(&self.checkpoint_signs);
        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
        let mut sign_observation = move |_: &[u8], _: &_, _: &_| {
            observation_signs.set(observation_signs.get() + 1);
            Ok(signature())
        };
        let mut sign_checkpoint = move |_: &[u8], _: &_, _: &_| {
            checkpoint_signs.set(checkpoint_signs.get() + 1);
            Ok(signature())
        };
        Ok(use_inputs(ResidentServiceNativePreparedInputs {
            current: self.current,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: self.observation_schema,
                signing_authority: self.signing_authority,
            },
            observation_ref: record_ref("retained/native-prepared-dispatched"),
            checkpoint: OperationCheckpointInput {
                reference: record_ref("retained/native-prepared-checkpoint"),
                id: ControlCheckpointIdV1::parse("retained-native-prepared-checkpoint")
                    .expect("bounded checkpoint identity"),
            },
            checkpoint_issued_at: UnixTimeV1::new(self.current.now.get() + 1)
                .expect("positive checkpoint time"),
            checkpoint_schema: self.checkpoint_schema,
            checkpoint_authority: self.checkpoint_authority,
            resolve_store: self.resolver,
            accumulator_limits: prepared_accumulator_limits(),
            maximum_selection_bytes: 1 << 20,
            maximum_record_bytes: 1 << 20,
            registry: self.registry,
            contract_set: self.contracts,
            admit_rotation: &mut admit_rotation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
        }))
    }
}

#[derive(Default)]
struct HoldingProviderTaskExecutor {
    accepts: usize,
    accepted: Vec<ResidentServiceProviderTask>,
}

impl ResidentServiceProviderTaskExecutor for HoldingProviderTaskExecutor {
    fn try_accept(
        &mut self,
        task: ResidentServiceProviderTask,
    ) -> Result<(), ResidentServiceProviderTask> {
        self.accepts += 1;
        self.accepted.push(task);
        Ok(())
    }

    fn try_completion(&mut self) -> Option<ResidentServiceProviderTaskCompletion> {
        None
    }
}

struct RetainedProviderPreparedInputs<'a> {
    current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    signing_authority: &'a SigningAuthorityBindingV1,
    observation_schema: &'a SchemaDigest,
    checkpoint_schema: &'a SchemaDigest,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    resolver: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    binding: &'a RequestBindingV1,
    attempt_id: &'a CapabilityAttemptIdV1,
    result_schema: &'a SchemaDigest,
    failure_schema: &'a SchemaDigest,
    contracts: &'a ContractSetV1,
    kind: &'a BoundedResidentTextV1,
    version: &'a BoundedResidentTextV1,
    clock: &'a dyn WallClockSourcePort,
    executor: &'a mut HoldingProviderTaskExecutor,
    evidence: &'a dyn crate::provider_invocation::ProviderInvocationEvidenceAdmission,
    observation_signs: Rc<Cell<usize>>,
    checkpoint_signs: Rc<Cell<usize>>,
    lends: Rc<Cell<usize>>,
}

struct ExpiredInvokeInputs<'a> {
    clock: &'a ExactExpiryClock,
    signing_authority: &'a SigningAuthorityBindingV1,
    observation_schema: &'a SchemaDigest,
    checkpoint_schema: &'a SchemaDigest,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    resolver: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    expiry_loans: Rc<Cell<usize>>,
    continuation_loans: Rc<Cell<usize>>,
    external_work_loans: Rc<Cell<usize>>,
    observation_signs: Rc<Cell<usize>>,
    checkpoint_signs: Rc<Cell<usize>>,
    interruption: Option<ExpiryContinuationInterruption>,
    identity_loans: Rc<RefCell<Vec<ExpiryInputIdentities>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpiryContinuationInterruption {
    ObservationSigning,
    CheckpointSigning,
    DatabaseCommitReadBack,
    AnchorCustody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpiryInputIdentities {
    diagnostic_ref: ResidentRecordRefV1,
    observation_ref: ResidentRecordRefV1,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint_id: ControlCheckpointIdV1,
}

impl ResidentServiceInvokeInputPort for ExpiredInvokeInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<&ControlRecordCoordinate>,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_pre_dispatch_expiry_preparation_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServicePreDispatchExpiryPreparationInputs<'input>) -> Output,
    {
        self.expiry_loans.set(self.expiry_loans.get() + 1);
        let identities = ExpiryInputIdentities {
            diagnostic_ref: record_ref("retained/invoke-expiry-diagnostic"),
            observation_ref: record_ref("retained/invoke-expiry-observation"),
            checkpoint_ref: record_ref("retained/invoke-expiry-checkpoint"),
            checkpoint_id: ControlCheckpointIdV1::parse("retained-invoke-expiry-checkpoint")
                .expect("bounded checkpoint identity"),
        };
        self.identity_loans.borrow_mut().push(identities.clone());
        let observation_signs = Rc::clone(&self.observation_signs);
        let checkpoint_signs = Rc::clone(&self.checkpoint_signs);
        let interruption = self.interruption;
        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
        let mut sign_observation = move |_: &[u8], _: &_, _: &_| {
            let attempt = observation_signs.get() + 1;
            observation_signs.set(attempt);
            if interruption == Some(ExpiryContinuationInterruption::ObservationSigning)
                && attempt == 1
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "pre-dispatch expiry observation",
                    invariant: "modeled observation signer refusal",
                });
            }
            Ok(signature())
        };
        let mut sign_checkpoint = move |_: &[u8], _: &_, _: &_| {
            let attempt = checkpoint_signs.get() + 1;
            checkpoint_signs.set(attempt);
            if interruption == Some(ExpiryContinuationInterruption::CheckpointSigning)
                && attempt == 1
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "pre-dispatch expiry checkpoint",
                    invariant: "modeled checkpoint signer refusal",
                });
            }
            Ok(signature())
        };
        Ok(use_inputs(
            ResidentServicePreDispatchExpiryPreparationInputs {
                clock: self.clock,
                diagnostic_ref: identities.diagnostic_ref,
                observation_ref: identities.observation_ref,
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: self.observation_schema,
                    signing_authority: self.signing_authority,
                },
                checkpoint: OperationCheckpointInput {
                    reference: identities.checkpoint_ref,
                    id: identities.checkpoint_id,
                },
                checkpoint_schema: self.checkpoint_schema,
                checkpoint_authority: self.checkpoint_authority,
                resolve_store: self.resolver,
                maximum_diagnostic_bytes: 65_536,
                maximum_record_bytes: 1 << 20,
                accumulator_limits: prepared_accumulator_limits(),
                admit_rotation: &mut admit_rotation,
                sign_observation: &mut sign_observation,
                sign_checkpoint: &mut sign_checkpoint,
            },
        ))
    }

    fn with_pre_dispatch_expiry_continuation_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(
            ResidentServicePreDispatchExpiryContinuationInputs<'input>,
        ) -> Output,
    {
        self.continuation_loans
            .set(self.continuation_loans.get() + 1);
        let checkpoint_signs = Rc::clone(&self.checkpoint_signs);
        let interruption = self.interruption;
        let mut sign_checkpoint = move |_: &[u8], _: &_, _: &_| {
            let attempt = checkpoint_signs.get() + 1;
            checkpoint_signs.set(attempt);
            if interruption == Some(ExpiryContinuationInterruption::CheckpointSigning)
                && attempt == 1
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "pre-dispatch expiry checkpoint",
                    invariant: "modeled checkpoint signer refusal",
                });
            }
            Ok(signature())
        };
        Ok(use_inputs(
            ResidentServicePreDispatchExpiryContinuationInputs {
                checkpoint_schema: self.checkpoint_schema,
                checkpoint_authority: self.checkpoint_authority,
                resolve_store: self.resolver,
                accumulator_limits: prepared_accumulator_limits(),
                sign_checkpoint: &mut sign_checkpoint,
            },
        ))
    }

    fn with_provider_prepared_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderPreparedInputs<'input>) -> Output,
    {
        self.external_work_loans
            .set(self.external_work_loans.get() + 1);
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_operation_resolution_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceOperationResolutionInputs<'input>) -> Output,
    {
        self.external_work_loans
            .set(self.external_work_loans.get() + 1);
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }
}

pub(in crate::resident::process_host) struct ConsultChildExpiryInputs<'a> {
    pub clock: &'a ExactExpiryClock,
    pub signing_authority: &'a SigningAuthorityBindingV1,
    pub observation_schema: &'a SchemaDigest,
    pub checkpoint_schema: &'a SchemaDigest,
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub resolver: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
}

/// Expire one admitted Consult child through the same retained Invoke adapter
/// used by the public service.
///
/// The caller must first prove the child's parent and roster lineage. This
/// helper supplies only expiry custody and asserts that resolution cannot open
/// native or provider work.
pub(in crate::resident::process_host) fn expire_consult_child_before_dispatch<
    WriterGuard,
    Authority,
>(
    recovered: Box<super::super::restart::ResidentProcessRecoveredOperationResolution>,
    process: &mut super::super::ResidentProcessHost<'_, WriterGuard>,
    access: super::super::ResidentDeploymentOperationAccess<'_>,
    authority: &mut Authority,
    inputs: ConsultChildExpiryInputs<'_>,
) where
    WriterGuard: pale_ports::PrincipalWriterGuardPort,
    Authority: AmbientSessionAuthorityPort,
{
    let ConsultChildExpiryInputs {
        clock,
        signing_authority,
        observation_schema,
        checkpoint_schema,
        checkpoint_authority,
        resolver,
    } = inputs;
    let expiry_loans = Rc::new(Cell::new(0));
    let continuation_loans = Rc::new(Cell::new(0));
    let external_work_loans = Rc::new(Cell::new(0));
    let observation_signs = Rc::new(Cell::new(0));
    let checkpoint_signs = Rc::new(Cell::new(0));
    let identity_loans = Rc::new(RefCell::new(Vec::new()));
    let mut retained = InvokeRecoveryAdapter::new(
        ExpiredInvokeInputs {
            clock,
            signing_authority,
            observation_schema,
            checkpoint_schema,
            checkpoint_authority,
            resolver,
            expiry_loans: Rc::clone(&expiry_loans),
            continuation_loans: Rc::clone(&continuation_loans),
            external_work_loans: Rc::clone(&external_work_loans),
            observation_signs: Rc::clone(&observation_signs),
            checkpoint_signs: Rc::clone(&checkpoint_signs),
            interruption: None,
            identity_loans: Rc::clone(&identity_loans),
        },
        Box::new(UnavailableResidentServicePublicationInputs),
        UnavailableResidentServiceDelegateInputs,
        (),
        (),
        1,
    );
    retained.retain_without_response(RetainedInvokePhase::OperationResolution(recovered));

    let turn = program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
        &mut retained,
        process,
        access,
        authority,
    );

    assert_eq!(turn, program::ResidentProgramRetainedPhaseTurn::Advanced);
    assert_eq!(retained.retained.iter().count(), 0);
    assert_eq!(expiry_loans.get(), 1);
    assert_eq!(continuation_loans.get(), 0);
    assert_eq!(clock.reads(), 1);
    assert_eq!(observation_signs.get(), 1);
    assert_eq!(checkpoint_signs.get(), 1);
    assert_eq!(identity_loans.borrow().len(), 1);
    assert_eq!(external_work_loans.get(), 0);
    assert_eq!(process.operations.outstanding_task_reservations(), 0);
}

impl ResidentServiceInvokeInputPort for RetainedProviderPreparedInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<&ControlRecordCoordinate>,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_pre_dispatch_expiry_preparation_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServicePreDispatchExpiryPreparationInputs<'input>) -> Output,
    {
        Ok(lend_not_expired_pre_dispatch_inputs(
            self.current.now,
            self.signing_authority,
            self.observation_schema,
            self.checkpoint_schema,
            self.checkpoint_authority,
            self.resolver,
            use_inputs,
        ))
    }

    fn with_provider_prepared_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderPreparedInputs<'input>) -> Output,
    {
        self.lends.set(self.lends.get() + 1);
        let observation_signs = Rc::clone(&self.observation_signs);
        let checkpoint_signs = Rc::clone(&self.checkpoint_signs);
        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
        let mut admit_reconciliation = |_: &_| Ok(());
        let mut sign_observation = move |_: &[u8], _: &_, _: &_| {
            observation_signs.set(observation_signs.get() + 1);
            Ok(signature())
        };
        let mut sign_checkpoint = move |_: &[u8], _: &_, _: &_| {
            checkpoint_signs.set(checkpoint_signs.get() + 1);
            Ok(signature())
        };
        let admission = provider_admission(ProviderAdmissionFixture {
            binding: self.binding,
            attempt_id: self.attempt_id,
            signing: self.signing_authority,
            result_schema: self.result_schema,
            failure_schema: self.failure_schema,
            contracts: self.contracts,
            kind: self.kind,
            version: self.version,
        });
        let invocation = ProviderRecoveryInvocation {
            current: self.current,
            signing: self.signing_authority,
            checkpoint_authority: self.checkpoint_authority,
            observation_schema: self.observation_schema,
            checkpoint_schema: self.checkpoint_schema,
            clock: self.clock,
            resolver: self.resolver,
        }
        .input("retained-provider-prepared");
        Ok(use_inputs(ResidentServiceProviderPreparedInputs {
            current: self.current,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: self.observation_schema,
                signing_authority: self.signing_authority,
            },
            observation_ref: record_ref("retained/provider-prepared-dispatched"),
            checkpoint: OperationCheckpointInput {
                reference: record_ref("retained/provider-prepared-checkpoint"),
                id: ControlCheckpointIdV1::parse("retained-provider-prepared-checkpoint")
                    .expect("bounded checkpoint identity"),
            },
            checkpoint_issued_at: UnixTimeV1::new(self.current.now.get() + 1)
                .expect("positive checkpoint time"),
            checkpoint_schema: self.checkpoint_schema,
            checkpoint_authority: self.checkpoint_authority,
            resolve_store: self.resolver,
            accumulator_limits: prepared_accumulator_limits(),
            maximum_selection_bytes: 1 << 20,
            maximum_record_bytes: 1 << 20,
            admission,
            invocation,
            evidence: self.evidence,
            executor: self.executor,
            admit_rotation: &mut admit_rotation,
            admit_reconciliation: &mut admit_reconciliation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
        }))
    }
}

fn admit_reconciliation(
    _: &ReconciliationFrontierReferenceV1,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn admit_rotation(
    _: &ResidentRecordRefV1,
    _: &ArtifactDigest,
    _: &ResidentRecordRefV1,
    _: &ArtifactDigest,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn sign_observation(
    _: &[u8],
    _: &OperationObservationPreimageV1,
    _: &OperationObservationDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
    Ok(signature())
}

fn sign_checkpoint(
    _: &[u8],
    _: &ControlStateCheckpointPreimageV1,
    _: &ControlStateCheckpointDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
    Ok(signature())
}

impl ResidentServiceInvokeInputPort for ProviderRecoveryInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<&ControlRecordCoordinate>,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_provider_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceProviderRecoveryInputs<'input>) -> Output,
    {
        self.lends.fetch_add(1, Ordering::SeqCst);
        let recording = self
            .recording
            .take()
            .expect("one exact provider recovery allocation");
        let mut admit_reconciliation = admit_reconciliation;
        let mut admit_rotation = admit_rotation;
        let mut sign_observation = sign_observation;
        let mut sign_checkpoint = sign_checkpoint;
        Ok(use_inputs(ResidentServiceProviderRecoveryInputs {
            recording,
            maximum_selection_bytes: 1024 * 1024,
            admit_reconciliation: &mut admit_reconciliation,
            admit_rotation: &mut admit_rotation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
        }))
    }
}

fn sign_result(
    _: &[u8],
    _: &CapabilityResultPreimageV1,
    _: &CapabilityResultDigest,
) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
    Ok(signature())
}

impl ResidentServiceInvokeInputPort for NativeRecoveryInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<&ControlRecordCoordinate>,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_native_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceNativeRecoveryInputs<'input>) -> Output,
    {
        let ordinal = self.lends.fetch_add(1, Ordering::SeqCst) + 1;
        let checkpoint = |phase: &str| crate::resident::OperationCheckpointInput {
            reference: record_ref(&format!("retained-native/{ordinal}/{phase}-checkpoint")),
            id: ControlCheckpointIdV1::parse(format!("retained-native-{ordinal}-{phase}"))
                .expect("bounded checkpoint identity"),
        };
        let mut admit_rotation = admit_rotation;
        let mut sign_observation = sign_observation;
        let mut sign_checkpoint = sign_checkpoint;
        let mut sign_result = sign_result;
        Ok(use_inputs(ResidentServiceNativeRecoveryInputs {
            current: self.current,
            registry: self.registry,
            contract_set: self.contracts,
            outcome_ref: record_ref(&format!("retained-native/{ordinal}/outcome")),
            outcome_observation_ref: record_ref(&format!(
                "retained-native/{ordinal}/outcome-observation"
            )),
            result_ref: record_ref(&format!("retained-native/{ordinal}/result")),
            diagnostic_ref: record_ref(&format!("retained-native/{ordinal}/diagnostic")),
            terminal_ref: record_ref(&format!("retained-native/{ordinal}/terminal")),
            outcome_checkpoint: checkpoint("outcome"),
            terminal_checkpoint: checkpoint("terminal"),
            clock: self.clock,
            resolve_store: self.resolver,
            checkpoint_schema: self.checkpoint_schema,
            checkpoint_authority: self.checkpoint_authority,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: self.observation_schema,
                signing_authority: self.current.generation.signed.signing_authority,
            },
            maximum_selection_bytes: 1 << 20,
            maximum_record_bytes: 1 << 20,
            maximum_diagnostic_bytes: 65_536,
            maximum_outcome_bytes: 65_536,
            accumulator_limits: AccumulatorPreparationLimits::new(
                8, 20_000, 20_000, 8_192, 200_000,
            )
            .expect("native accumulator limits"),
            admit_rotation: &mut admit_rotation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
            sign_result: &mut sign_result,
        }))
    }
}

#[test]
fn retained_provider_resolution_keeps_one_exact_attempt_across_anchor_interruption() {
    let seen = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(0));
    let counted_seen = Rc::clone(&seen);
    let counted_completed = Rc::clone(&completed);
    with_process_host_provider_resolved_restart_fixture(&mut move |fixture| {
        let case = counted_seen.replace(counted_seen.get() + 1);
        if case != 0 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
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
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                super::super::ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("provider-resolved head classifies");
        let super::super::restart::ResidentProcessOperationRestartDisposition::ProviderResolved(
            recovered,
        ) = disposition
        else {
            panic!("provider resolution has one retained owner")
        };

        let now = current
            .at_time(UnixTimeV1::new(current.now.get() + 10).expect("positive recovery time"));
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
        let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let kind = BoundedResidentTextV1::parse("host_compute").expect("bounded kind");
        let version = BoundedResidentTextV1::parse("1").expect("bounded version");
        let resolver = SelectedStore(selected_store.clone());
        let random = RetainedRandom::new();
        let observation_signs = Rc::new(Cell::new(0));
        let checkpoint_signs = Rc::new(Cell::new(0));
        let lends = Rc::new(Cell::new(0));
        let mut retained = InvokeRecoveryAdapter::new(
            RetainedProviderResolvedInputs {
                session,
                current: &now,
                kind: &kind,
                version: &version,
                result_schema: &result_schema,
                failure_schema: &failure_schema,
                contracts: &contracts,
                signing_authority,
                observation_schema: &observation_schema,
                checkpoint_schema: &checkpoint_schema,
                checkpoint_authority,
                resolver: &resolver,
                random: &random,
                observation_signs: Rc::clone(&observation_signs),
                checkpoint_signs: Rc::clone(&checkpoint_signs),
                lends: Rc::clone(&lends),
            },
            Box::new(UnavailableResidentServicePublicationInputs),
            UnavailableResidentServiceDelegateInputs,
            (),
            (),
            1,
        );
        retained.retain_without_response(RetainedInvokePhase::ProviderResolved(recovered));
        let mut append = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        append_anchor.fail_next_advance();

        let first = program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
            &mut retained,
            &mut host,
            super::super::ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
        );
        assert_eq!(first, program::ResidentProgramRetainedPhaseTurn::Advanced);
        assert!(matches!(
            retained.retained.iter().next(),
            Some(RetainedServicePhase::Invoke(
                RetainedInvokePhase::ProviderResolvedContinuation(_)
            ))
        ));
        assert_eq!(retained.retained.iter().count(), 1);
        assert_eq!(append.counts().0, 1, "one successor is committed");
        assert_eq!(observation_signs.get(), 1);
        assert_eq!(checkpoint_signs.get(), 1);
        let random_calls = random.calls();

        let second = program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
            &mut retained,
            &mut host,
            super::super::ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
        );
        assert_eq!(second, program::ResidentProgramRetainedPhaseTurn::Advanced);
        assert_eq!(retained.retained.iter().count(), 0);
        assert_eq!(append.counts().0, 1, "anchor continuation cannot re-commit");
        assert_eq!(observation_signs.get(), 1, "continuation cannot re-sign");
        assert_eq!(checkpoint_signs.get(), 1, "continuation cannot re-sign");
        assert_eq!(random.calls(), random_calls, "attempt identity is retained");
        assert_eq!(
            lends.get(),
            1,
            "the retained anchor continuation needs no second physical-input loan"
        );
        counted_completed.set(counted_completed.get() + 1);
    });
    assert_eq!(seen.get(), 3);
    assert_eq!(completed.get(), 1);
}

#[test]
fn retained_preparations_continue_exact_dispatch_custody_without_replay() {
    let native_cases = Rc::new(Cell::new(0));
    let provider_cases = Rc::new(Cell::new(0));
    let counted_native = Rc::clone(&native_cases);
    let counted_provider = Rc::clone(&provider_cases);
    with_process_host_prepared_restart_fixtures(&mut move |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current,
            request,
            native_registry,
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
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                super::super::ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("retained preparation classifies");
        let recovery_current = current
            .at_time(UnixTimeV1::new(current.now.get() + 10).expect("positive recovery time"));
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let resolver = SelectedStore(selected_store.clone());
        let observation_signs = Rc::new(Cell::new(0));
        let checkpoint_signs = Rc::new(Cell::new(0));
        let lends = Rc::new(Cell::new(0));
        let mut append = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        append_anchor.fail_next_advance();

        match disposition {
            super::super::restart::ResidentProcessOperationRestartDisposition::NativePrepared(
                attempt,
            ) => {
                counted_native.set(counted_native.get() + 1);
                let mut retained = InvokeRecoveryAdapter::new(
                    RetainedNativePreparedInputs {
                        current: &recovery_current,
                        signing_authority,
                        observation_schema: &observation_schema,
                        checkpoint_schema: &checkpoint_schema,
                        checkpoint_authority,
                        resolver: &resolver,
                        registry: native_registry,
                        contracts: &contracts,
                        observation_signs: Rc::clone(&observation_signs),
                        checkpoint_signs: Rc::clone(&checkpoint_signs),
                        lends: Rc::clone(&lends),
                    },
                    Box::new(UnavailableResidentServicePublicationInputs),
                    UnavailableResidentServiceDelegateInputs,
                    (),
                    (),
                    1,
                );
                retained.retain_without_response(RetainedInvokePhase::NativePrepared(attempt));
                let first =
                    program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
                        &mut retained,
                        &mut host,
                        super::super::ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody: &mut append,
                            sessions,
                            anchor: &mut append_anchor,
                        },
                        &mut ProvingAuthority,
                    );
                assert_eq!(first, program::ResidentProgramRetainedPhaseTurn::Advanced);
                assert!(matches!(
                    retained.retained.iter().next(),
                    Some(RetainedServicePhase::Invoke(
                        RetainedInvokePhase::NativePreparedContinuation(_)
                    ))
                ));
                assert_eq!(retained.retained.iter().count(), 1);
                assert_eq!(observation_signs.get(), 1);
                assert_eq!(checkpoint_signs.get(), 1);
                assert_eq!(append.counts().0, 1);

                let second =
                    program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
                        &mut retained,
                        &mut host,
                        super::super::ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody: &mut append,
                            sessions,
                            anchor: &mut append_anchor,
                        },
                        &mut ProvingAuthority,
                    );
                assert_eq!(second, program::ResidentProgramRetainedPhaseTurn::Advanced);
                assert_eq!(retained.retained.iter().count(), 0);
                assert_eq!(append.counts().0, 1, "continuation cannot re-commit");
                assert_eq!(observation_signs.get(), 1, "continuation cannot re-sign");
                assert_eq!(checkpoint_signs.get(), 1, "continuation cannot re-sign");
                assert_eq!(lends.get(), 2);
                assert_eq!(host.operations.outstanding_task_reservations(), 0);
            }
            super::super::restart::ResidentProcessOperationRestartDisposition::ProviderPrepared(
                attempt,
            ) => {
                counted_provider.set(counted_provider.get() + 1);
                let OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared { attempt_id, .. },
                } = attempt.start.control.observation().selected().1.state()
                else {
                    panic!("provider fixture retains one prepared attempt")
                };
                let attempt_id = attempt_id.clone();
                let binding = attempt
                    .start
                    .control
                    .operation()
                    .operation()
                    .handle()
                    .1
                    .request_binding()
                    .clone();
                assert_eq!(
                    binding,
                    request
                        .local_request_binding()
                        .expect("provider request binding")
                );
                let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
                let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
                let kind = BoundedResidentTextV1::parse("host_compute").expect("bounded kind");
                let version = BoundedResidentTextV1::parse("1").expect("bounded version");
                let clock = RecoveryReleaseClock(AtomicU64::new(
                    recovery_current
                        .now
                        .get()
                        .checked_add(1)
                        .expect("modeled checkpoint time"),
                ));
                let evidence = UnusedRecoveredProviderEvidence;
                let mut executor = HoldingProviderTaskExecutor::default();
                let mut retained = InvokeRecoveryAdapter::new(
                    RetainedProviderPreparedInputs {
                        current: &recovery_current,
                        signing_authority,
                        observation_schema: &observation_schema,
                        checkpoint_schema: &checkpoint_schema,
                        checkpoint_authority,
                        resolver: &resolver,
                        binding: &binding,
                        attempt_id: &attempt_id,
                        result_schema: &result_schema,
                        failure_schema: &failure_schema,
                        contracts: &contracts,
                        kind: &kind,
                        version: &version,
                        clock: &clock,
                        executor: &mut executor,
                        evidence: &evidence,
                        observation_signs: Rc::clone(&observation_signs),
                        checkpoint_signs: Rc::clone(&checkpoint_signs),
                        lends: Rc::clone(&lends),
                    },
                    Box::new(UnavailableResidentServicePublicationInputs),
                    UnavailableResidentServiceDelegateInputs,
                    (),
                    (),
                    1,
                );
                retained.retain_without_response(RetainedInvokePhase::ProviderPrepared(attempt));
                let first =
                    program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
                        &mut retained,
                        &mut host,
                        super::super::ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody: &mut append,
                            sessions,
                            anchor: &mut append_anchor,
                        },
                        &mut ProvingAuthority,
                    );
                assert_eq!(first, program::ResidentProgramRetainedPhaseTurn::Advanced);
                assert!(matches!(
                    retained.retained.iter().next(),
                    Some(RetainedServicePhase::Invoke(
                        RetainedInvokePhase::ProviderPreparedContinuation(_)
                    ))
                ));
                assert_eq!(observation_signs.get(), 1);
                assert_eq!(checkpoint_signs.get(), 1);
                assert_eq!(append.counts().0, 1);

                let second =
                    program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
                        &mut retained,
                        &mut host,
                        super::super::ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody: &mut append,
                            sessions,
                            anchor: &mut append_anchor,
                        },
                        &mut ProvingAuthority,
                    );
                assert_eq!(second, program::ResidentProgramRetainedPhaseTurn::Advanced);
                assert_eq!(retained.retained.iter().count(), 0);
                assert_eq!(append.counts().0, 1, "continuation cannot re-commit");
                assert_eq!(observation_signs.get(), 1, "continuation cannot re-sign");
                assert_eq!(checkpoint_signs.get(), 1, "continuation cannot re-sign");
                assert_eq!(lends.get(), 2);
                drop(retained);
                assert_eq!(
                    executor.accepts, 1,
                    "one durable dispatch releases one task"
                );
                assert_eq!(executor.accepted.len(), 1);
            }
            _ => panic!("fixture has one prepared owner"),
        }
    });
    assert_eq!(native_cases.get(), 1);
    assert_eq!(provider_cases.get(), 1);
}

#[test]
fn pre_dispatch_expiry_blocks_retained_provider_execution() {
    let provider_cases = Rc::new(Cell::new(0));
    let counted_provider = Rc::clone(&provider_cases);
    with_process_host_prepared_restart_fixtures(&mut move |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
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
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                super::super::ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("retained preparation classifies");
        let super::super::restart::ResidentProcessOperationRestartDisposition::ProviderPrepared(
            attempt,
        ) = disposition
        else {
            return;
        };
        counted_provider.set(counted_provider.get() + 1);

        let expires_at = *attempt
            .start
            .control
            .operation()
            .operation()
            .handle()
            .1
            .expires_at();
        let clock = ExactExpiryClock::new(expires_at);
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let resolver = SelectedStore(selected_store.clone());
        let expiry_loans = Rc::new(Cell::new(0));
        let continuation_loans = Rc::new(Cell::new(0));
        let external_work_loans = Rc::new(Cell::new(0));
        let observation_signs = Rc::new(Cell::new(0));
        let checkpoint_signs = Rc::new(Cell::new(0));
        let identity_loans = Rc::new(RefCell::new(Vec::new()));
        let mut retained = InvokeRecoveryAdapter::new(
            ExpiredInvokeInputs {
                clock: &clock,
                signing_authority,
                observation_schema: &observation_schema,
                checkpoint_schema: &checkpoint_schema,
                checkpoint_authority,
                resolver: &resolver,
                expiry_loans: Rc::clone(&expiry_loans),
                continuation_loans: Rc::clone(&continuation_loans),
                external_work_loans: Rc::clone(&external_work_loans),
                observation_signs: Rc::clone(&observation_signs),
                checkpoint_signs: Rc::clone(&checkpoint_signs),
                interruption: None,
                identity_loans: Rc::clone(&identity_loans),
            },
            Box::new(UnavailableResidentServicePublicationInputs),
            UnavailableResidentServiceDelegateInputs,
            (),
            (),
            1,
        );
        retained.retain_without_response(RetainedInvokePhase::ProviderPrepared(attempt));
        let mut append = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);

        let turn = program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
            &mut retained,
            &mut host,
            super::super::ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
        );

        assert_eq!(turn, program::ResidentProgramRetainedPhaseTurn::Advanced);
        assert_eq!(retained.retained.iter().count(), 0);
        assert_eq!(expiry_loans.get(), 1);
        assert_eq!(continuation_loans.get(), 0);
        assert_eq!(clock.reads(), 1);
        assert_eq!(observation_signs.get(), 1);
        assert_eq!(checkpoint_signs.get(), 1);
        assert_eq!(identity_loans.borrow().len(), 1);
        assert_eq!(append.counts(), (1, 1));
        assert_eq!(
            external_work_loans.get(),
            0,
            "expiry must precede provider access"
        );
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(provider_cases.get(), 1);
}

#[test]
fn retained_expiry_continues_exact_custody_after_each_phase_failure() {
    for interruption in [
        ExpiryContinuationInterruption::ObservationSigning,
        ExpiryContinuationInterruption::CheckpointSigning,
        ExpiryContinuationInterruption::DatabaseCommitReadBack,
        ExpiryContinuationInterruption::AnchorCustody,
    ] {
        let provider_cases = Rc::new(Cell::new(0));
        let counted_provider = Rc::clone(&provider_cases);
        with_process_host_prepared_restart_fixtures(&mut move |fixture| {
            let ProcessHostRestartFixture {
                recovery,
                admission,
                signing_authority,
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
            let port = MemoryAuthorityStore::default();
            let mut host = host_with_generation(&port, &selector, &selected_store, generation);
            let restart = restart_input(recovery);
            let disposition = host
                .classify_existing_operation(
                    super::super::ResidentProcessExistingOperationStart {
                        selector: selector.clone(),
                        evidence: Box::new(evidence),
                    },
                    association(signing_authority, &restart),
                    super::super::ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody,
                        sessions,
                        anchor,
                    },
                    restart,
                    admission,
                )
                .expect("retained preparation classifies");
            let super::super::restart::ResidentProcessOperationRestartDisposition::ProviderPrepared(
                attempt,
            ) = disposition
            else {
                return;
            };
            counted_provider.set(counted_provider.get() + 1);

            let expected_locator =
                operation_locator_from_operation(attempt.start.control.operation().operation());
            let expires_at = *attempt
                .start
                .control
                .operation()
                .operation()
                .handle()
                .1
                .expires_at();
            let clock = ExactExpiryClock::new(expires_at);
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let resolver = SelectedStore(selected_store.clone());
            let expiry_loans = Rc::new(Cell::new(0));
            let continuation_loans = Rc::new(Cell::new(0));
            let external_work_loans = Rc::new(Cell::new(0));
            let observation_signs = Rc::new(Cell::new(0));
            let checkpoint_signs = Rc::new(Cell::new(0));
            let identity_loans = Rc::new(RefCell::new(Vec::new()));
            let mut retained = InvokeRecoveryAdapter::new(
                ExpiredInvokeInputs {
                    clock: &clock,
                    signing_authority,
                    observation_schema: &observation_schema,
                    checkpoint_schema: &checkpoint_schema,
                    checkpoint_authority,
                    resolver: &resolver,
                    expiry_loans: Rc::clone(&expiry_loans),
                    continuation_loans: Rc::clone(&continuation_loans),
                    external_work_loans: Rc::clone(&external_work_loans),
                    observation_signs: Rc::clone(&observation_signs),
                    checkpoint_signs: Rc::clone(&checkpoint_signs),
                    interruption: Some(interruption),
                    identity_loans: Rc::clone(&identity_loans),
                },
                Box::new(UnavailableResidentServicePublicationInputs),
                UnavailableResidentServiceDelegateInputs,
                (),
                (),
                1,
            );
            retained.retain_without_response(RetainedInvokePhase::ProviderPrepared(attempt));
            let mut append = AppendCustody::committing(custody);
            let mut append_anchor = AppendAnchor::available(anchor);
            match interruption {
                ExpiryContinuationInterruption::DatabaseCommitReadBack => {
                    append.push_read_back(OperationCustodyReadBackOutcome::Predecessor);
                    append.fail_next_commit(ControlStoreErrorKind::OutcomeUnknown);
                    append.fail_next_read_back(ControlStoreErrorKind::Storage);
                    append.push_read_back(OperationCustodyReadBackOutcome::Predecessor);
                    append.push_read_back(OperationCustodyReadBackOutcome::ExactSuccessor);
                }
                ExpiryContinuationInterruption::AnchorCustody => {
                    append_anchor.fail_next_advance();
                    append_anchor.fail_next_advance();
                }
                ExpiryContinuationInterruption::ObservationSigning
                | ExpiryContinuationInterruption::CheckpointSigning => {}
            }

            let mut retained_expiry_identity = None;
            for turn_index in 0..4 {
                let turn =
                    program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
                        &mut retained,
                        &mut host,
                        super::super::ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody: &mut append,
                            sessions,
                            anchor: &mut append_anchor,
                        },
                        &mut ProvingAuthority,
                    );
                assert_eq!(
                    external_work_loans.get(),
                    0,
                    "{interruption:?} turn {turn_index} cannot open native or provider work"
                );
                if retained.retained.iter().next().is_none() {
                    assert_eq!(turn, program::ResidentProgramRetainedPhaseTurn::Advanced);
                    break;
                }
                match retained.retained.iter().next() {
                    Some(RetainedServicePhase::Invoke(RetainedInvokePhase::ProviderPrepared(
                        prepared,
                    ))) => {
                        assert_eq!(
                            interruption,
                            ExpiryContinuationInterruption::ObservationSigning,
                            "only a failed observation signature retains the recovered phase"
                        );
                        assert_eq!(
                            operation_locator_from_operation(
                                prepared.start.control.operation().operation()
                            ),
                            expected_locator
                        );
                        assert_eq!(turn, program::ResidentProgramRetainedPhaseTurn::Deferred);
                    }
                    Some(RetainedServicePhase::Invoke(RetainedInvokePhase::PreDispatchExpiry(
                        expiry,
                    ))) => {
                        assert_eq!(expiry.locator, expected_locator);
                        assert_eq!(expiry.observed_at, expires_at);
                        let identity = (
                            expiry.key.clone(),
                            expiry.selector.clone(),
                            expiry.locator.clone(),
                        );
                        match &retained_expiry_identity {
                            Some(expected) => assert_eq!(&identity, expected),
                            None => retained_expiry_identity = Some(identity),
                        }
                    }
                    _ => panic!("{interruption:?} must retain only its exact expiry phase"),
                }
            }

            assert!(
                retained.retained.iter().next().is_none(),
                "{interruption:?} must complete within the bounded modeled turns"
            );
            assert_eq!(external_work_loans.get(), 0);
            assert_eq!(host.operations.outstanding_task_reservations(), 0);
            let loans = identity_loans.borrow();
            assert_eq!(loans.len(), expiry_loans.get());
            assert!(!loans.is_empty());
            assert!(
                loans.windows(2).all(|pair| pair[0] == pair[1]),
                "{interruption:?} must reuse the same expiry identities"
            );
            match interruption {
                ExpiryContinuationInterruption::ObservationSigning => {
                    assert_eq!(expiry_loans.get(), 2);
                    assert_eq!(continuation_loans.get(), 0);
                    assert_eq!(
                        (
                            clock.reads(),
                            observation_signs.get(),
                            checkpoint_signs.get()
                        ),
                        (2, 2, 1)
                    );
                    assert_eq!(append.counts(), (1, 1));
                    assert_eq!(append_anchor.counts(), (1, 1));
                }
                ExpiryContinuationInterruption::CheckpointSigning => {
                    assert_eq!(expiry_loans.get(), 1);
                    assert_eq!(continuation_loans.get(), 1);
                    assert_eq!(
                        (
                            clock.reads(),
                            observation_signs.get(),
                            checkpoint_signs.get()
                        ),
                        (1, 1, 2)
                    );
                    assert_eq!(append.counts(), (1, 1));
                    assert_eq!(append_anchor.counts(), (1, 1));
                }
                ExpiryContinuationInterruption::DatabaseCommitReadBack => {
                    assert_eq!(expiry_loans.get(), 1);
                    assert!(continuation_loans.get() > 0);
                    assert_eq!(
                        (
                            clock.reads(),
                            observation_signs.get(),
                            checkpoint_signs.get()
                        ),
                        (1, 1, 1)
                    );
                    assert_eq!(append.counts(), (1, 4));
                    assert_eq!(append_anchor.counts(), (1, 1));
                }
                ExpiryContinuationInterruption::AnchorCustody => {
                    assert_eq!(expiry_loans.get(), 1);
                    assert!(continuation_loans.get() > 0);
                    assert_eq!(
                        (
                            clock.reads(),
                            observation_signs.get(),
                            checkpoint_signs.get()
                        ),
                        (1, 1, 1)
                    );
                    assert_eq!(append.counts(), (1, 1));
                    assert_eq!(append_anchor.counts(), (3, 1));
                }
            }
        });
        assert_eq!(
            provider_cases.get(),
            1,
            "{interruption:?} must exercise the single provider-prepared fixture"
        );
    }
}

#[test]
fn retained_provider_dispatch_records_uncertainty_without_replaying_the_provider() {
    let cases = Rc::new(AtomicUsize::new(0));
    let counted_cases = Rc::clone(&cases);
    with_process_host_provider_recovery_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
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
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                super::super::ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("protected provider classification");
        let super::super::restart::ResidentProcessOperationRestartDisposition::Provider(attempt) =
            disposition
        else {
            return;
        };
        counted_cases.fetch_add(1, Ordering::SeqCst);

        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let clock = IncreasingClock(AtomicU64::new(current.now.get() + 20));
        let resolver = SelectedStore(selected_store.clone());
        let lends = Rc::new(AtomicUsize::new(0));
        let recording = ProviderRecoveryInvocation {
            current,
            signing: signing_authority,
            checkpoint_authority,
            observation_schema: &observation_schema,
            checkpoint_schema: &checkpoint_schema,
            clock: &clock,
            resolver: &resolver,
        }
        .input("retained-turn");
        let mut retained = InvokeRecoveryAdapter::new(
            ProviderRecoveryInputs {
                recording: Some(recording),
                lends: Rc::clone(&lends),
            },
            Box::new(UnavailableResidentServicePublicationInputs),
            UnavailableResidentServiceDelegateInputs,
            (),
            (),
            1,
        );
        retained.retain_without_response(RetainedInvokePhase::ProviderDispatched(attempt));
        assert!(
            program::ResidentProgramRootRecoveryInputPort::has_runnable_retained_invoke(&retained),
            "the program scheduler must see the retained provider dispatch"
        );

        let mut append = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        let turn = program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
            &mut retained,
            &mut host,
            super::super::ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
        );

        assert_eq!(turn, program::ResidentProgramRetainedPhaseTurn::Advanced);
        assert_eq!(lends.load(Ordering::SeqCst), 1);
        assert_eq!(append.counts(), (1, 1));
        assert_eq!(append_anchor.counts(), (1, 1));
        assert_eq!(retained.retained.iter().count(), 0);
        assert!(
            !program::ResidentProgramRootRecoveryInputPort::has_runnable_retained_invoke(&retained),
            "durable uncertainty releases the retained scheduler slot"
        );
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(
            host.provider_routes.checked_out, 0,
            "provider recovery cannot check out a provider call"
        );
        assert!(host.provider_routes.active.is_empty());
    });
    assert_eq!(cases.load(Ordering::SeqCst), 2);
}

#[test]
fn retained_native_dispatch_continues_recording_without_reopening_the_handler() {
    let cases = Rc::new(AtomicUsize::new(0));
    let counted_cases = Rc::clone(&cases);
    with_process_host_native_recovery_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
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
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                super::super::ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("protected native classification");
        let super::super::restart::ResidentProcessOperationRestartDisposition::Native(attempt) =
            disposition
        else {
            return;
        };
        let recovered = attempt.start.control.observation();
        if !matches!(
            (
                recovered.selected().1.state(),
                recovered
                    .selected_predecessor()
                    .map(|(_, observation)| observation.state()),
            ),
            (
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched { .. }
                },
                Some(OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared { .. }
                })
            )
        ) {
            return;
        }
        counted_cases.fetch_add(1, Ordering::SeqCst);

        let registry = NativeActionRegistry::new().expect("sealed native registry");
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let resolver = SelectedStore(selected_store.clone());
        let floor = current
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
        let clock = FailOnceClock::new(floor.get(), 4);
        let lends = Rc::new(AtomicUsize::new(0));
        let mut retained = InvokeRecoveryAdapter::new(
            NativeRecoveryInputs {
                current,
                registry: &registry,
                contracts: &contracts,
                checkpoint_authority,
                observation_schema: &observation_schema,
                checkpoint_schema: &checkpoint_schema,
                clock: &clock,
                resolver: &resolver,
                lends: Rc::clone(&lends),
            },
            Box::new(UnavailableResidentServicePublicationInputs),
            UnavailableResidentServiceDelegateInputs,
            (),
            (),
            1,
        );
        retained.retain_without_response(RetainedInvokePhase::NativeDispatched(attempt));

        let first = program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
            &mut retained,
            &mut host,
            super::super::ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut ProvingAuthority,
        );
        assert_eq!(first, program::ResidentProgramRetainedPhaseTurn::Advanced);
        assert_eq!(
            clock.reads(),
            4,
            "the modeled interruption follows the handler"
        );
        assert!(matches!(
            retained.retained.iter().next(),
            Some(RetainedServicePhase::Invoke(
                RetainedInvokePhase::NativeRecording(recovery)
            )) if matches!(&recovery.state, RetainedNativeRecordingState::Unstaged(_))
        ));

        let mut append_store = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);

        for _ in 0..12 {
            if !program::ResidentProgramRootRecoveryInputPort::has_runnable_retained_invoke(
                &retained,
            ) {
                break;
            }
            let turn = program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
                &mut retained,
                &mut host,
                super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_store,
                    sessions,
                    anchor: &mut append_anchor,
                },
                &mut ProvingAuthority,
            );
            assert_ne!(turn, program::ResidentProgramRetainedPhaseTurn::Idle);
        }
        assert_eq!(retained.retained.iter().count(), 0);
        assert!(lends.load(Ordering::SeqCst) >= 2);
        assert!(
            !program::ResidentProgramRootRecoveryInputPort::has_runnable_retained_invoke(&retained),
            "terminal custody releases the native recording slot"
        );
    });
    assert_eq!(cases.load(Ordering::SeqCst), 1);
}

#[path = "retained_hook_response_expiry.rs"]
mod hook_response_expiry;
