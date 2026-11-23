//! Child-specific policy, signer and collection admissions for service recovery.

use super::*;
use crate::resident::process_host::operation_request_host;

type SignedHandles = Rc<RefCell<Vec<(OperationHandlePreimageV1, OperationHandleDigest)>>>;
pub(super) type SavedCollection = Rc<RefCell<Option<ConsultationCollection>>>;
pub(super) type InterruptedParent = Rc<RefCell<Option<ControlRecordCoordinate>>>;
pub(super) type InterruptedChild = Rc<RefCell<Option<OperationHandlePreimageV1>>>;

pub(super) struct ChildPolicy {
    pub(super) base: ConformanceConsultPolicy,
    pub(super) request: CommonRequestEnvelopeV1,
    pub(super) start: ResidentServiceOperationStartPolicy,
}

impl ResidentServiceInvokePolicyPort for ChildPolicy {
    fn evaluate_invoke(
        &mut self,
        context: ResidentServiceInvokePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        self.base.evaluate_invoke(context)
    }
}

impl ResidentServiceConsultPolicyPort for ChildPolicy {
    type StopInput = <ConformanceConsultPolicy as ResidentServiceConsultPolicyPort>::StopInput;

    fn evaluate_consult(
        &mut self,
        context: ResidentServiceConsultPolicyContext<'_>,
    ) -> Result<ResidentServiceConsultPolicy, InvalidResidentRecordV1> {
        let mut policy = self.base.evaluate_consult(context)?;
        assert_eq!(policy.roster.slots.len(), 1);
        policy.roster.slots[0].source = ResidentServiceConsultationRosterSource::FreshChild {
            request: Box::new(self.request.clone()),
            operation_class: OperationClassV1::Invoke,
            policy: Box::new(self.start.clone()),
        };
        Ok(policy)
    }

    fn evaluate_consult_completion(
        &mut self,
        context: ResidentServiceConsultCompletionContext<'_>,
    ) -> Result<ResidentServiceConsultDecision<Self::StopInput>, InvalidResidentRecordV1> {
        self.base.evaluate_consult_completion(context)
    }
}

pub(super) struct ChildCryptography {
    pub(super) template: ControlStateCheckpointPreimageV1,
    pub(super) handles: SignedHandles,
    pub(super) counters: Arc<ConsultConformanceCounters>,
    pub(super) interrupt_before_child: Option<InterruptedChild>,
    pub(super) interrupt_after_child_signature: Option<InterruptedChild>,
    pub(super) signing_available: bool,
}

impl ResidentServiceInvokeCryptographyPort for ChildCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &AdmittedAmbientSession,
        _: &AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &LocalOperationPublicationPolicyInput<'_>,
        _: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn sign_handle(
        &self,
        _: &[u8],
        preimage: &OperationHandlePreimageV1,
        digest: &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        assert!(
            self.signing_available,
            "cold collection recovery must not sign a handle"
        );
        if preimage.parent_operation.is_some() {
            if let Some(interrupted) = &self.interrupt_before_child {
                *interrupted.borrow_mut() = Some(preimage.clone());
                return Err(invalid(
                    "Consult child signature",
                    "child signing is unavailable",
                ));
            }
            if let Some(interrupted) = &self.interrupt_after_child_signature {
                *interrupted.borrow_mut() = Some(preimage.clone());
            }
        }
        let mut handles = self.handles.borrow_mut();
        assert!(
            !handles.iter().any(|(_, existing)| existing == digest),
            "a saved parent or child must not be signed twice"
        );
        handles.push((preimage.clone(), digest.clone()));
        self.counters
            .handle_signatures
            .fetch_add(1, Ordering::SeqCst);
        Ok(signature())
    }

    fn sign_observation(
        &self,
        _: &[u8],
        _: &OperationObservationPreimageV1,
        _: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        assert!(
            self.signing_available,
            "cold collection recovery must not sign an observation"
        );
        if self
            .interrupt_after_child_signature
            .as_ref()
            .is_some_and(|child| child.borrow().is_some())
        {
            return Err(invalid(
                "Consult child observation",
                "interrupted after handle signing",
            ));
        }
        Ok(signature())
    }

    fn prepare_checkpoint(
        &self,
        prepared: &PreparedLocalOperationCheckpoint,
        id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError> {
        let mut checkpoint = self.template.clone();
        if let Some((reference, previous)) = prepared.predecessor() {
            checkpoint.domain_sequence =
                ControlDomainSequenceV1::new(previous.domain_sequence().get() + 1).unwrap();
            checkpoint.previous_control_checkpoint_ref = Some(reference.clone());
            checkpoint.previous_control_checkpoint_sha256 =
                Some(previous.canonical_control_checkpoint_sha256().clone());
        }
        checkpoint.control_checkpoint_id = id.clone();
        checkpoint.committed_transaction_sequence = prepared.transaction_sequence();
        checkpoint.state_root_sha256 = prepared.state_root().clone();
        checkpoint.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
        checkpoint.changed_leaf_keys = prepared.changed_keys().to_vec();
        checkpoint.leaf_count = prepared.leaf_count();
        Ok(checkpoint)
    }

    fn sign_checkpoint(
        &self,
        _: &[u8],
        _: &ControlStateCheckpointPreimageV1,
        _: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        assert!(
            self.signing_available,
            "cold collection recovery must not sign a checkpoint"
        );
        Ok(signature())
    }
}

type AdmitCollection =
    Box<dyn FnMut(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HistoricalAdmissionMode {
    Missing,
    Refused,
    Allowed,
}

pub(super) struct ChildInputs<'a> {
    base: ConformanceConsultInputs<'a>,
    admit: AdmitCollection,
    historical: Option<AdmitCollection>,
    historical_mode: Option<Rc<std::cell::Cell<HistoricalAdmissionMode>>>,
    interruption: Option<InterruptedParent>,
}

impl<'a> ChildInputs<'a> {
    pub(super) fn new(
        base: ConformanceConsultInputs<'a>,
        handles: SignedHandles,
        saved: SavedCollection,
    ) -> Self {
        Self {
            base,
            historical: None,
            historical_mode: None,
            interruption: None,
            admit: Box::new(move |collection| {
                let handles = handles.borrow();
                assert_eq!(
                    handles.len(),
                    2,
                    "one parent and one actual child were signed"
                );
                assert_eq!(handles[0].0.operation_class, OperationClassV1::Consult);
                assert_eq!(handles[1].0.operation_class, OperationClassV1::Invoke);
                let parent = handles[1].0.parent_operation.as_ref().unwrap();
                assert_eq!(parent.operation_handle_ref, collection.parent().record_ref);
                assert_eq!(
                    parent.operation_handle_sha256.as_str(),
                    handles[0].1.as_str()
                );
                assert_eq!(
                    collection.parent().record_sha256.as_str(),
                    handles[0].1.as_str()
                );
                let [slot] = collection.slots() else {
                    panic!("exactly one fresh child slot")
                };
                let ConsultationContributionSource::FreshChild { child } = slot.roster().source()
                else {
                    panic!("the saved slot must identify a fresh child")
                };
                assert_eq!(child.record_sha256.as_str(), handles[1].1.as_str());
                assert!(matches!(
                    slot.disposition(),
                    ConsultationSlotDisposition::Outstanding
                ));
                let mut saved = saved.borrow_mut();
                if let Some(expected) = saved.as_ref() {
                    assert_eq!(
                        collection, expected,
                        "cold recovery must retain the exact child and parent collection"
                    );
                } else {
                    *saved = Some(collection.clone());
                }
                Ok(())
            }),
        }
    }

    pub(super) fn interrupt_before_collection(mut self, parent: InterruptedParent) -> Self {
        self.interruption = Some(parent);
        self
    }

    pub(super) fn with_historical_collection(
        mut self,
        saved: SavedCollection,
        mode: Rc<std::cell::Cell<HistoricalAdmissionMode>>,
        attempts: Rc<std::cell::Cell<usize>>,
    ) -> Self {
        self.historical_mode = Some(Rc::clone(&mode));
        self.historical = Some(Box::new(move |collection| {
            attempts.set(attempts.get() + 1);
            if mode.get() == HistoricalAdmissionMode::Allowed
                && saved.borrow().as_ref() == Some(collection)
            {
                Ok(())
            } else {
                Err(invalid("historical collection", "the saved roster changed"))
            }
        }));
        self
    }
}

impl ResidentServiceConsultInputPort for ChildInputs<'_> {
    type Work = ();
    type Output = ();
    type ChildRecoveryError = ();
    type RetentionFailure = ();
    type Executor = ConformanceConsultExecutor;
    type StoreResolver = ConformanceConsultStoreResolver;
    type AdmitCollection = AdmitCollection;
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
            Option<&mut HistoricalConsultationAdmission<'_>>,
        ) -> Return,
    {
        let admit = &mut self.admit;
        let historical_collection = &mut self.historical;
        let historical_mode = &self.historical_mode;
        self.base.with_finite_turn_inputs(
            |limits, tasks, resolver, _, executor, recover, retain, historical| {
                let historical = historical_collection
                    .as_mut()
                    .filter(|_| {
                        historical_mode
                            .as_ref()
                            .is_some_and(|mode| mode.get() != HistoricalAdmissionMode::Missing)
                    })
                    .map(|value| &mut **value as &mut HistoricalConsultationAdmission<'_>)
                    .or(historical);
                use_inputs(
                    limits, tasks, resolver, admit, executor, recover, retain, historical,
                )
            },
        )
    }

    fn with_initial_collection_inputs<Return, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        parent: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServiceConsultInitialCollectionInputs<'input>) -> Return,
    {
        if let Some(interruption) = &self.interruption {
            *interruption.borrow_mut() = Some(parent.clone());
            return Err(ResidentOperationFrameFailure::ResidentUnavailable);
        }
        let admit = &mut self.admit;
        self.base
            .with_initial_collection_inputs(deployment, parent, |inputs| {
                use_inputs(ResidentServiceConsultInitialCollectionInputs {
                    observation_ref: inputs.observation_ref,
                    observed_at: inputs.observed_at,
                    signing: inputs.signing,
                    checkpoint: inputs.checkpoint,
                    resolve_store: inputs.resolve_store,
                    admit_collection: &mut **admit,
                    admit_rotation: inputs.admit_rotation,
                    sign_observation: inputs.sign_observation,
                    sign_checkpoint: inputs.sign_checkpoint,
                })
            })
    }

    fn with_stop_inputs<Return, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        parent: &ControlRecordCoordinate,
        meaning: &ResidentServiceConsultStopMeaning,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultStopInputs<'input>) -> Return,
    {
        self.base
            .with_stop_inputs(deployment, parent, meaning, use_inputs)
    }

    fn with_completion_join_inputs<Return, UseInputs>(
        &mut self,
        selection: ResidentServiceConsultJoinSelection<'_>,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultJoinInputs<'input>) -> Return,
    {
        self.base.with_completion_join_inputs(selection, use_inputs)
    }

    fn with_terminal_child_join_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultTerminalChildInputs<'input>) -> Return,
    {
        self.base.with_terminal_child_join_inputs(use_inputs)
    }
}

/// Read the original parent and child from protected custody at the interruption.
pub(super) fn snapshot_child_prefix(
    state: &ConformanceResidentState,
    request: &CommonRequestEnvelopeV1,
    handles: &SignedHandles,
    parent: &ControlRecordCoordinate,
    custody: &mut dyn OperationCustodyPort,
    anchors: &mut ConformanceAntiRollbackAnchors,
) -> (ResidentOperationFrameRecords, Option<ControlRecord>) {
    let mut inputs = ConformanceInvokeInputs::new_for_consult(
        &state.store,
        &state.historical,
        &state.session,
        request,
    )
    .with_background_at(conformance_ingress_time());
    inputs
        .with_recovery_inputs(Some(parent), |inputs| {
            with_child_admission(inputs.recovery_admission, handles, |admission| {
                let control = recover_operation_control(
                    OperationControlRecoveryInput {
                        operation: LocalOperationHandleRecoveryInput {
                            context: inputs.operation,
                            expected_handle: parent,
                            maximum_parent_depth: 1,
                        },
                        observation: OperationObservationSelection::SelectedHead,
                        observation_limits: inputs.limits.observation,
                        checkpoint_limits: inputs.limits.checkpoints,
                        membership_limits: inputs.limits.membership,
                    },
                    custody,
                    &state.store_owner,
                    anchors,
                    admission,
                )
                .expect("the saved parent has protected custody");
                let operation = control.operation().operation();
                let records = operation_request_host::issued_start_records(operation).unwrap();
                let intent = operation
                    .consultation_start_intent()
                    .expect("the original roster is saved");
                let mut children = intent.fresh_children();
                let child = children
                    .next()
                    .expect("one child was planned")
                    .handle_coordinate();
                assert!(children.next().is_none());
                let record = custody
                    .read_operation_record(&state.store, &child, MAXIMUM_RECORD_BYTES)
                    .unwrap();
                (records, record)
            })
        })
        .unwrap()
}

pub(super) struct ChildRecoveryInputs<'a> {
    pub(super) base: ConformanceInvokeInputs<'a>,
    pub(super) handles: SignedHandles,
}

fn with_child_admission<Output>(
    admission: OperationControlRecoveryAdmission<'_>,
    handles: &SignedHandles,
    use_admission: impl FnOnce(OperationControlRecoveryAdmission<'_>) -> Output,
) -> Output {
    let mut admit = |handle: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
        let signed = handles.borrow();
        let known = signed.iter().any(|(expected, _)| expected == handle);
        let lineage = match (&handle.parent_operation, parent) {
            (None, None) => true,
            (Some(link), Some(parent)) => {
                link.operation_handle_sha256 == *parent.canonical_operation_handle_sha256()
                    && link.operation_handle_sha256.as_str() == signed[0].1.as_str()
            }
            _ => false,
        };
        (known && lineage).then_some(()).ok_or_else(|| {
            invalid(
                "Consult child recovery",
                "historical handle or exact parent changed",
            )
        })
    };
    use_admission(OperationControlRecoveryAdmission {
        admit_request: admission.admit_request,
        admit_handle_authority: &mut admit,
        verify_handle_signature: admission.verify_handle_signature,
        admit_checkpoint: admission.admit_checkpoint,
        resolve_observation_signer: admission.resolve_observation_signer,
        admit_observation_state: admission.admit_observation_state,
        admit_rotation: admission.admit_rotation,
        verify_observation_signature: admission.verify_observation_signature,
        verify_checkpoint_signature: admission.verify_checkpoint_signature,
    })
}

impl ResidentServiceInvokeInputPort for ChildRecoveryInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        request: &CommonRequestEnvelopeV1,
        locator: Option<&ControlRecordCoordinate>,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        let handles = &self.handles;
        self.base
            .with_invoke_inputs(deployment, request, locator, |inputs| {
                with_child_admission(inputs.recovery_admission, handles, |admission| {
                    use_inputs(ResidentServiceInvokeRecoveryInputs {
                        operation: inputs.operation,
                        maximum_parent_depth: 1,
                        limits: inputs.limits,
                        recovery_admission: admission,
                        checkpoint_authority: inputs.checkpoint_authority,
                    })
                })
            })
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        deployment: &ResidentServiceDeploymentSelector,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        let handles = &self.handles;
        self.base
            .with_background_invoke_inputs(deployment, |inputs| {
                with_child_admission(inputs.recovery_admission, handles, |admission| {
                    use_inputs(ResidentServiceInvokeBackgroundRecoveryInputs {
                        committed_observation_limits: inputs.committed_observation_limits,
                        operation: inputs.operation,
                        maximum_parent_depth: 1,
                        limits: inputs.limits,
                        recovery_admission: admission,
                        maximum_session_basis_bytes: inputs.maximum_session_basis_bytes,
                        observed_at: inputs.observed_at,
                    })
                })
            })
    }
}
