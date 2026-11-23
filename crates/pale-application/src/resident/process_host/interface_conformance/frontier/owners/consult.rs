//! Evidence-resolved Consult policy and bounded physical input owners.

use super::*;

pub(in super::super::super) struct ConformanceConsultStop;

impl ResidentServiceConsultStopInput for ConformanceConsultStop {
    fn stop_meaning(&self) -> ResidentServiceConsultStopMeaning {
        unreachable!("Continue(None) cannot select a Consult stop")
    }
}

pub(in super::super::super) struct ConformanceConsultPolicy {
    start: Option<ResidentServiceConsultPolicy>,
    completion_policy: ImmutableRecordReferenceV1,
    counters: Arc<ConsultConformanceCounters>,
}

impl ResidentServiceInvokePolicyPort for ConformanceConsultPolicy {
    fn evaluate_invoke(
        &mut self,
        _: ResidentServiceInvokePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        Err(invalid(
            "resident Invoke policy",
            "the Consult conformance frontier does not admit Invoke",
        ))
    }
}

impl ResidentServiceConsultPolicyPort for ConformanceConsultPolicy {
    type StopInput = ConformanceConsultStop;

    fn evaluate_consult(
        &mut self,
        _: ResidentServiceConsultPolicyContext<'_>,
    ) -> Result<ResidentServiceConsultPolicy, InvalidResidentRecordV1> {
        self.counters.start_policy.fetch_add(1, Ordering::SeqCst);
        self.start.take().ok_or_else(|| {
            invalid(
                "resident Consult policy",
                "the deterministic start policy was already consumed",
            )
        })
    }

    fn evaluate_consult_completion(
        &mut self,
        context: ResidentServiceConsultCompletionContext<'_>,
    ) -> Result<ResidentServiceConsultDecision<Self::StopInput>, InvalidResidentRecordV1> {
        if context.completion_policy != &self.completion_policy {
            return Err(invalid(
                "resident Consult completion policy",
                "the retained completion policy changed",
            ));
        }
        self.counters
            .completion_policy
            .fetch_add(1, Ordering::SeqCst);
        Ok(ResidentServiceConsultDecision::Continue(None))
    }
}

pub(in super::super::super) fn consult_policy(
    lease: &OperationLeaseV1,
    request: &CommonRequestEnvelopeV1,
    counters: Arc<ConsultConformanceCounters>,
) -> ConformanceConsultPolicy {
    let completion_policy = consult_completion_policy_reference();
    let evidence = consult_evidence_reference();
    let roster = ResidentServiceConsultationRosterPolicy {
        completion_policy: completion_policy.clone(),
        opened_at: conformance_ingress_time(),
        deadline: UnixTimeV1::new(80).expect("Consult collection deadline"),
        slots: vec![conformance_consult_roster_slot(evidence)],
        limits: ConsultationCollectionLimits::new(1, 65_536, 65_536)
            .expect("bounded Consult collection"),
    };
    ConformanceConsultPolicy {
        start: Some(ResidentServiceConsultPolicy {
            parent: invoke_policy(lease, request),
            roster,
        }),
        completion_policy,
        counters,
    }
}

fn consult_completion_policy_reference() -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref("interface-conformance-consult-completion-policy"),
        record_sha256: ArtifactDigest::hash_bytes(b"interface-conformance Consult completion"),
    }
}

fn consult_evidence_reference() -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref("interface-conformance-consult-evidence"),
        record_sha256: ArtifactDigest::hash_bytes(b"interface-conformance Consult evidence"),
    }
}

fn conformance_consult_roster_slot(
    evidence: ImmutableRecordReferenceV1,
) -> ResidentServiceConsultationRosterSlot {
    ResidentServiceConsultationRosterSlot {
        slot_id: ConsultationSlotId::parse("existing-evidence").expect("Consult slot identity"),
        requirement: ConsultationRequirement::Required,
        requirement_evidence: evidence.clone(),
        participant: ParticipantRefV1::Application {
            application_id: "interface-conformance-evidence".into(),
        },
        provider_binding: None,
        context: None,
        source: ResidentServiceConsultationRosterSource::EvidenceResolved { evidence },
    }
}

pub(in super::super::super) struct ConformanceConsultStoreResolver {
    store: AuthorityStoreCoordinate,
}

impl OperationObservationAuthorityStoreResolver for ConformanceConsultStoreResolver {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(self.store.clone())
    }
}

pub(in super::super::super) struct ConformanceConsultExecutor {
    counters: Arc<ConsultConformanceCounters>,
}

impl ResidentServiceConsultTaskExecutor<(), ()> for ConformanceConsultExecutor {
    fn try_accept(
        &mut self,
        task: ResidentServiceConsultTask<()>,
    ) -> Result<(), ResidentServiceConsultTask<()>> {
        self.counters
            .child_stop_join_or_acceptance_attempts
            .fetch_add(1, Ordering::SeqCst);
        Err(task)
    }

    fn try_completion(&mut self) -> Option<ResidentServiceConsultTaskCompletion<(), ()>> {
        self.counters.executor_polls.fetch_add(1, Ordering::SeqCst);
        None
    }
}

fn admit_conformance_collection(
    collection: &ConsultationCollection,
) -> Result<(), InvalidResidentRecordV1> {
    let evidence = consult_evidence_reference();
    let expected_roster = ConsultationRosterSlot::new(
        ConsultationSlotId::parse("existing-evidence").expect("Consult slot identity"),
        ConsultationRequirement::Required,
        evidence.clone(),
        ParticipantRefV1::Application {
            application_id: "interface-conformance-evidence".into(),
        },
        None,
        None,
        ConsultationContributionSource::EvidenceResolved {
            evidence: evidence.clone(),
        },
    );
    let expected_disposition = ConsultationSlotDisposition::Fulfilled {
        evidence,
        recorded_at: conformance_ingress_time(),
    };
    let [slot] = collection.slots() else {
        return Err(invalid(
            "resident Consult collection",
            "the conformance roster must contain exactly one slot",
        ));
    };
    if collection.completion_policy() != &consult_completion_policy_reference()
        || slot.roster() != &expected_roster
        || slot.disposition() != &expected_disposition
    {
        return Err(invalid(
            "resident Consult collection",
            "the evidence-resolved roster or its binding changed",
        ));
    }
    Ok(())
}

fn unreachable_consult_child_recovery(
    _: &ConsultationChildDispatchCandidate,
) -> Result<RecoveredOperationObservation, ()> {
    unreachable!("an evidence-resolved Consult has no child to recover")
}

fn unreachable_consult_child_retention(
    _: &ConsultationChildDispatchCandidate,
    _: RecoveredOperationObservation,
    _: &mut dyn OperationCustodyPort,
    _: &mut dyn AntiRollbackAnchorPort,
) -> ConsultationChildRetentionOutcome<(), ()> {
    unreachable!("an evidence-resolved Consult has no child to retain")
}

type ConformanceChildRetention = fn(
    &ConsultationChildDispatchCandidate,
    RecoveredOperationObservation,
    &mut dyn OperationCustodyPort,
    &mut dyn AntiRollbackAnchorPort,
) -> ConsultationChildRetentionOutcome<(), ()>;

type CollectionAdmission =
    Box<dyn FnMut(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>>;

pub(in super::super::super) struct ConformanceConsultInputs<'a> {
    store: &'a AuthorityStoreCoordinate,
    session: &'a ConformanceSessionFixture,
    domain: ControlCheckpointDomainIdV1,
    checkpoint: ConformanceCheckpointAuthority,
    resolver: ConformanceConsultStoreResolver,
    admit_collection: CollectionAdmission,
    historical_admission: Option<CollectionAdmission>,
    recover_child:
        fn(&ConsultationChildDispatchCandidate) -> Result<RecoveredOperationObservation, ()>,
    retain_child: ConformanceChildRetention,
    pub(super) counters: Arc<ConsultConformanceCounters>,
}

impl<'a> ConformanceConsultInputs<'a> {
    pub(in super::super::super) fn new(
        store: &'a AuthorityStoreCoordinate,
        session: &'a ConformanceSessionFixture,
        request: &CommonRequestEnvelopeV1,
        counters: Arc<ConsultConformanceCounters>,
    ) -> Self {
        Self {
            store,
            session,
            domain: SupportedOperationCheckpointProfile::conflict_domain(request)
                .expect("request conflict domain"),
            checkpoint: ConformanceCheckpointAuthority::new(session),
            resolver: ConformanceConsultStoreResolver {
                store: store.clone(),
            },
            admit_collection: Box::new(admit_conformance_collection),
            historical_admission: None,
            recover_child: unreachable_consult_child_recovery,
            retain_child: unreachable_consult_child_retention,
            counters,
        }
    }
    pub(in super::super::super) fn expect_expiry_recovery(
        mut self,
        previous: ConsultationCollection,
        stopped: ConsultationCollection,
    ) -> Self {
        assert_ne!(previous, stopped, "historical frontiers must be distinct");
        let expected = stopped.clone();
        let counters = Arc::clone(&self.counters);
        self.admit_collection = Box::new(move |collection| {
            assert_eq!(
                collection, &expected,
                "current admission keeps the exact frozen stop"
            );
            counters
                .current_stopped_collections
                .fetch_add(1, Ordering::SeqCst);
            admit_conformance_collection(collection)
        });
        let counters = Arc::clone(&self.counters);
        self.historical_admission = Some(Box::new(move |collection| {
            if collection == &previous {
                counters
                    .historical_previous_collections
                    .fetch_add(1, Ordering::SeqCst);
            } else {
                assert_eq!(collection, &stopped);
                counters
                    .historical_stopped_collections
                    .fetch_add(1, Ordering::SeqCst);
            }
            admit_conformance_collection(collection)
        }));
        self
    }
}

impl ResidentServiceConsultInputPort for ConformanceConsultInputs<'_> {
    type Work = ();
    type Output = ();
    type ChildRecoveryError = ();
    type RetentionFailure = ();
    type Executor = ConformanceConsultExecutor;
    type StoreResolver = ConformanceConsultStoreResolver;
    type AdmitCollection = CollectionAdmission;
    type RecoverChild =
        fn(&ConsultationChildDispatchCandidate) -> Result<RecoveredOperationObservation, ()>;
    type RetainChild = fn(
        &ConsultationChildDispatchCandidate,
        RecoveredOperationObservation,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
    ) -> ConsultationChildRetentionOutcome<(), ()>;

    fn with_finite_turn_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            ConsultationCollectionLimits,
            ResidentServiceConsultTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut ResidentServiceConsultExecutor<Self::Executor>,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return,
    {
        self.counters.finite_turn.fetch_add(1, Ordering::SeqCst);
        let mut executor = ResidentServiceConsultExecutor::new(ConformanceConsultExecutor {
            counters: Arc::clone(&self.counters),
        });
        Ok(use_inputs(
            ConsultationCollectionLimits::new(1, 65_536, 65_536)
                .expect("bounded Consult collection"),
            ResidentServiceConsultTaskLimits {
                maximum_in_flight: NonZeroUsize::new(1).expect("one Consult task"),
                maximum_batch_members: NonZeroUsize::new(1).expect("one Consult member"),
            },
            &self.resolver,
            &mut self.admit_collection,
            &mut executor,
            &mut self.recover_child,
            &mut self.retain_child,
            self.historical_admission.as_mut().map(|admit| {
                &mut **admit as &mut crate::resident::HistoricalConsultationAdmission<'_>
            }),
        ))
    }

    fn with_initial_collection_inputs<Return, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServiceConsultInitialCollectionInputs<'input>) -> Return,
    {
        self.counters
            .initial_collection
            .fetch_add(1, Ordering::SeqCst);
        let observation_schema = schema(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let checkpoint_authority = self
            .checkpoint
            .context(self.store, &self.domain, self.session);
        let signing = SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &observation_schema,
            signing_authority: &self.session.signing,
        };
        let checkpoint = ConsultationSuccessorCommitInput {
            accumulator_limits: accumulator_limits(),
            identity: OperationCheckpointInput {
                reference: record_ref("interface-conformance-consult-checkpoint"),
                id: ControlCheckpointIdV1::parse("interface-conformance-consult-checkpoint")
                    .expect("Consult checkpoint identity"),
            },
            issued_at: conformance_ingress_time(),
            schema: &checkpoint_schema,
            authority: &checkpoint_authority,
        };
        let mut admit_collection = admit_conformance_collection;
        let mut admit_rotation = |old_ref: &ResidentRecordRefV1,
                                  old_sha: &ArtifactDigest,
                                  new_ref: &ResidentRecordRefV1,
                                  new_sha: &ArtifactDigest| {
            (old_ref == new_ref && old_sha == new_sha)
                .then_some(())
                .ok_or_else(|| invalid("Consult observation", "signing authority changed"))
        };
        let mut sign_observation =
            |_: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
                Ok(signature())
            };
        let mut sign_checkpoint =
            |_: &[u8], _: &ControlStateCheckpointPreimageV1, _: &ControlStateCheckpointDigest| {
                Ok(signature())
            };
        Ok(use_inputs(ResidentServiceConsultInitialCollectionInputs {
            observation_ref: record_ref("interface-conformance-consult-observation"),
            observed_at: conformance_ingress_time(),
            signing,
            checkpoint,
            resolve_store: &self.resolver,
            admit_collection: &mut admit_collection,
            admit_rotation: &mut admit_rotation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
        }))
    }

    fn with_stop_inputs<Return, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ControlRecordCoordinate,
        _: &ResidentServiceConsultStopMeaning,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultStopInputs<'input>) -> Return,
    {
        self.counters
            .child_stop_join_or_acceptance_attempts
            .fetch_add(1, Ordering::SeqCst);
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_completion_join_inputs<Return, UseInputs>(
        &mut self,
        _: ResidentServiceConsultJoinSelection<'_>,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultJoinInputs<'input>) -> Return,
    {
        self.counters
            .child_stop_join_or_acceptance_attempts
            .fetch_add(1, Ordering::SeqCst);
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_terminal_child_join_inputs<Return, UseInputs>(
        &mut self,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultTerminalChildInputs<'input>) -> Return,
    {
        self.counters
            .child_stop_join_or_acceptance_attempts
            .fetch_add(1, Ordering::SeqCst);
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }
}
