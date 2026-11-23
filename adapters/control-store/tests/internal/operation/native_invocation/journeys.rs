//! Admit and dispatch native computation before checking its retained outcome.

use super::super::local_recovery;
use super::*;

pub(super) fn run_native_action_journeys(journeys: &[Journey]) {
    let copies = journeys
        .iter()
        .filter(|journey| matches!(journey, Journey::HookResponse(case) if case.is_expiry()))
        .count()
        * 2
        + journeys
            .iter()
            .filter(|journey| {
                matches!(
                    journey,
                    Journey::ConsultExpiry
                        | Journey::ConsultExpiryWithIntent
                        | Journey::ChildExpiry(_)
                        | Journey::ConsultChildExpiry(_)
                        | Journey::ConsultSweepChildExpiry(_)
                        | Journey::ConsultSweepInvokeExpiry(_)
                )
            })
            .count();
    local_recovery::with_generation_copies(copies, |generation, effective, generations| {
        for &journey in journeys {
            let replay = matches!(journey, Journey::DispatchReplay);
            let cold_hook = matches!(journey, Journey::HookResponse(case) if case.is_cold());
            let context = match journey {
                Journey::HookResponse(case) if case.is_cold() => LocalStartContext::ObservedNative,
                Journey::UnusedReadAuthority
                | Journey::Service(service::Case::UnusedReadAuthority)
                | Journey::ReadAuthorityCancellationWins
                | Journey::ReadAuthorityDispatchWins => LocalStartContext::ReadAuthorityOnly,
                Journey::EvidenceOnly | Journey::Service(service::Case::EvidenceObligations) => {
                    LocalStartContext::EvidenceOnly
                }
                Journey::ReadAndEvidence => LocalStartContext::Inherited,
                _ => LocalStartContext::SelfContained,
            };
            let registry = NativeActionRegistry::new().unwrap();
            with_start_context_class(
                generation,
                effective,
                "native-sqlite",
                |request| {
                    if cold_hook {
                        service::hook_request_body(generation, effective, request);
                    } else {
                        native_request_body(request);
                    }
                    if matches!(
                        journey,
                        Journey::HandlerFailure
                            | Journey::StagedRecoveryFailure
                            | Journey::StagedFailureDiagnosticSubstitution
                            | Journey::StagedFailureRequestSubstitution
                            | Journey::StagedFailureRequestKindSubstitution
                            | Journey::StagedFailureRequestMissing
                            | Journey::FailureAfterUnknown
                            | Journey::ControlFailureAfterCancellation
                            | Journey::Service(service::Case::HandlerFailure)
                    ) {
                        request.body.payload["payload"]["paths"] =
                            serde_json::json!(["../token=private"]);
                    }
                    request.body.payload_sha256 = ArtifactDigest::hash_bytes(
                        pale_core::canon::to_canonical_string(&request.body.payload)
                            .unwrap()
                            .as_bytes(),
                    );
                },
                if matches!(journey, Journey::ConsultExpiryWithIntent) {
                    StartAdmission::EvidenceConsult {
                        build_collection: consult_expiry::collection_admission,
                        limits: ConsultationCollectionLimits::new(4, 1 << 20, 1 << 20).unwrap(),
                    }
                } else if cold_hook
                    || matches!(
                        journey,
                        Journey::ConsultChildExpiry(_)
                            | Journey::ConsultSweepChildExpiry(_)
                            | Journey::ConsultExpiry
                            | Journey::ConsultExpiryWithIntent
                    )
                {
                    StartAdmission::AcceptedOwnerModel
                } else {
                    StartAdmission::NativeInput {
                        registry: &registry,
                        reject_owner: false,
                    }
                },
                context,
                if matches!(
                    journey,
                    Journey::ConsultChildExpiry(_)
                        | Journey::ConsultSweepChildExpiry(_)
                        | Journey::ConsultExpiry
                        | Journey::ConsultExpiryWithIntent
                ) {
                    OperationClassV1::Consult
                } else {
                    OperationClassV1::Invoke
                },
                false,
                None,
                |prepared, request, owner_calls, record_signatures, parents| {
                    assert_eq!((owner_calls, record_signatures), (1, 2));
                    assert_eq!(
                        parents.session.lease().read_ceiling().is_empty(),
                        !context.has_read_authority()
                    );
                    assert_eq!(
                        parents.session.lease().evidence_obligations().is_empty(),
                        !context.has_evidence_obligations()
                    );
                    assert_eq!(
                        parents.lease.read_set().is_empty(),
                        !context.has_read_authority()
                    );
                    assert_eq!(
                        parents.lease.evidence_obligations().is_empty(),
                        !context.has_evidence_obligations()
                    );
                    let directory = crate::test_support::private_tempdir();
                    let binding = parents.current.deployment_binding;
                    let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
                        ResidentWorkspaceIdV1::parse(
                            binding.deployment_terms().workspace_id().as_str(),
                        )
                        .unwrap(),
                        parents.current.deployment_binding_ref.clone(),
                        ArtifactDigest::parse(binding.canonical_binding_sha256().as_str()).unwrap(),
                    ));
                    let location = ControlStorageRoot::retain(directory.path())
                        .unwrap()
                        .prepare_store_location(scope.clone())
                        .unwrap();
                    let coordinate = store_coordinate();
                    let initialized = initialize_unadmitted_store(
                        &location,
                        ControlStoreIdentity::new(
                            scope,
                            coordinate.store_id().clone(),
                            coordinate.incarnation_id().clone(),
                        ),
                    )
                    .unwrap();
                    let file = initialized.path();
                    let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                        &location,
                        coordinate.clone(),
                    )
                    .unwrap();
                    let session = parents.session.lease();
                    let body = SessionLeaseBody::new(
                        coordinate.clone(),
                        session.session_admission_lease_digest().clone(),
                        session.schema_id().clone(),
                        session.schema_sha256().clone(),
                        ControlRecordBytes::new(
                            pale_core::canon::to_canonical_string(
                                &SessionAdmissionLeaseWireV1::from(session),
                            )
                            .unwrap()
                            .into_bytes(),
                        )
                        .unwrap(),
                    );
                    assert_eq!(
                        sessions.compare_and_retain(&body).unwrap(),
                        SessionLeaseBodyRetention::Retained
                    );
                    drop(sessions);
                    let mut sql = connection(file);
                    let mut port = SqliteOperationCustody {
                        connection: &mut sql,
                        coordinate: coordinate.clone(),
                    };
                    retain_operation_lease_body(
                        &mut port,
                        &coordinate,
                        &record_ref("start-lease"),
                        parents.lease,
                        1 << 20,
                    )
                    .unwrap();
                    let prepared = prepared
                        .unwrap()
                        .prepare_checkpoint(
                            AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000)
                                .unwrap(),
                            &mut port,
                        )
                        .unwrap();
                    let mut fields = checkpoint_preimage(1, None);
                    fields.checkpoint_domain_id =
                        SupportedOperationCheckpointProfile::conflict_domain(request).unwrap();
                    fields.checkpoint_profile_sha256 =
                        SupportedOperationCheckpointProfile::content_digest().unwrap();
                    fields.committed_transaction_sequence = prepared.transaction_sequence();
                    fields.state_root_sha256 = prepared.state_root().clone();
                    fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
                    fields.changed_leaf_keys = prepared.changed_keys().to_vec();
                    fields.leaf_count = ControlLeafCountV1::new(4).unwrap();
                    fields.issued_at = UnixTimeV1::new(61).unwrap();
                    let (physical, pending) = prepared
                        .issue_checkpoint(
                            &record_ref("operation/checkpoint-1"),
                            fields.clone(),
                            &fields.schema_sha256,
                            &checkpoint_authority(&fields),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap();
                    let mut anchor = IndependentAnchor::default();
                    let initial = physical
                        .commit(&mut port, &mut anchor, 1 << 20)
                        .unwrap()
                        .advance_anchor(&mut anchor)
                        .unwrap();
                    let initial_candidate = initial.candidate().clone();
                    let initial_checkpoint =
                        read_checkpoint(initial_candidate.checkpoint().successor_record(), &fields);
                    let issued_operation = pending
                        .finish(initial)
                        .unwrap_or_else(|_| panic!("actual initial custody"));
                    let (handle_ref, handle) = issued_operation.handle();
                    let first_record = initial_candidate
                        .records()
                        .iter()
                        .find_map(|record| match record {
                            OperationCustodyRecord::OperationObservation(value) => Some(value),
                            _ => None,
                        })
                        .unwrap();
                    let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                    let first = issue_observation(
                        parents.signing,
                        handle_ref,
                        handle,
                        schema.clone(),
                        OperationObservationRevisionV1::new(1).unwrap(),
                        None,
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Resolving,
                        },
                        60,
                    );
                    assert_eq!(
                        first_record.canonical_record_jcs().as_slice(),
                        pale_core::canon::to_canonical_string(&first)
                            .unwrap()
                            .as_bytes()
                    );
                    if matches!(
                        journey,
                        Journey::ConsultExpiry | Journey::ConsultExpiryWithIntent
                    ) {
                        let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                            &location,
                            coordinate.clone(),
                        )
                        .unwrap();
                        consult_expiry::run(
                            &issued_operation,
                            parents.signing,
                            (first_record.coordinate().record_ref(), &first),
                            (
                                initial_candidate
                                    .checkpoint()
                                    .successor_record()
                                    .coordinate()
                                    .record_ref(),
                                &initial_checkpoint,
                            ),
                            &fields,
                            file,
                            &mut port,
                            &mut anchor,
                            request,
                            &parents,
                            generations.pop().unwrap(),
                            &sessions,
                            matches!(journey, Journey::ConsultExpiryWithIntent),
                        );
                        return;
                    }
                    if let Journey::ChildExpiry(depth)
                    | Journey::ConsultChildExpiry(depth)
                    | Journey::ConsultSweepChildExpiry(depth)
                    | Journey::ConsultSweepInvokeExpiry(depth) = journey
                    {
                        let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                            &location,
                            coordinate.clone(),
                        )
                        .unwrap();
                        child_expiry::run(
                            child_start::Fixture {
                                operation: &issued_operation,
                                request,
                                parents: &parents,
                                generation,
                                checkpoint: &fields,
                                file,
                                registry: &registry,
                            },
                            (first_record.coordinate().record_ref(), &first),
                            generations.pop().unwrap(),
                            &sessions,
                            &mut port,
                            &mut anchor,
                            if matches!(
                                journey,
                                Journey::ConsultSweepChildExpiry(_)
                                    | Journey::ConsultSweepInvokeExpiry(_)
                            ) {
                                child_expiry::Sweep::Consult(depth)
                            } else {
                                child_expiry::Sweep::Provider(depth)
                            },
                        );
                        return;
                    }
                    let recovered_operation = if matches!(journey, Journey::RecoveredService) {
                        drop(port);
                        drop(sql);
                        let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                            &location,
                            coordinate.clone(),
                        )
                        .unwrap();
                        sql = connection(file);
                        port = SqliteOperationCustody {
                            connection: &mut sql,
                            coordinate: coordinate.clone(),
                        };
                        Some(history::recover_saved(
                            &history::Input {
                                operation: &issued_operation,
                                coordinate: &coordinate,
                                selected_member: (first_record.coordinate().record_ref(), &first),
                                known: &[(first_record.coordinate().record_ref(), &first)],
                                checkpoint: &fields,
                                signing: parents.signing,
                            },
                            request,
                            &parents,
                            &sessions,
                            &mut port,
                            &mut anchor,
                        ))
                    } else {
                        None
                    };
                    let operation = recovered_operation
                        .as_ref()
                        .map_or(&issued_operation, RecoveredLocalOperation::operation);
                    let (handle_ref, handle) = operation.handle();
                    let initial_observation = OperationObservationAdmissionContextV1 {
                        signed: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &schema,
                            signing_authority: parents.signing,
                        },
                        handle_ref,
                        handle,
                        predecessor: Some((first_record.coordinate().record_ref(), &first)),
                    };
                    let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
                    let context_binding =
                        matches!(journey, Journey::EvidenceOnly | Journey::ReadAndEvidence).then(
                            || {
                                let evidence = ImmutableRecordReferenceV1 {
                                    record_ref: record_ref("context/projection"),
                                    record_sha256: ArtifactDigest::hash_bytes(
                                        b"admitted context projection",
                                    ),
                                };
                                let admissions = AtomicUsize::new(0);
                                let signs = AtomicUsize::new(0);
                                let prepared = prepare_operation_context_binding(
                                    OperationContextBindingInput {
                                        operation,
                                        request,
                                        session: parents.session,
                                        current: parents.current,
                                        lease: parents.lease,
                                        evidence: evidence.clone(),
                                        observation_ref: record_ref("context/bound"),
                                        observation: &initial_observation,
                                    },
                                    &mut ProvingAuthority,
                                    |actual, obligations| {
                                        admissions.fetch_add(1, Ordering::SeqCst);
                                        assert_eq!(actual, &evidence);
                                        assert_eq!(obligations, &[record_ref("evidence-a")]);
                                        Ok(())
                                    },
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        signs.fetch_add(1, Ordering::SeqCst);
                                        Ok(signature())
                                    },
                                )
                                .unwrap();
                                assert_eq!(admissions.load(Ordering::SeqCst), 1);
                                assert_eq!(signs.load(Ordering::SeqCst), 1);
                                let (physical, fields) = prepare_append(
                                    prepared.prepare_append(&resolve).unwrap(),
                                    operation,
                                    (
                                        initial_candidate
                                            .checkpoint()
                                            .successor()
                                            .coordinate()
                                            .record_ref(),
                                        &initial_checkpoint,
                                    ),
                                    &mut port,
                                    62,
                                );
                                let completed = physical
                                    .commit(&mut port, &mut anchor)
                                    .unwrap()
                                    .advance_anchor(&mut anchor)
                                    .unwrap();
                                assert_eq!(
                                    completed.database_evidence(),
                                    OperationCustodyDatabaseEvidence::New
                                );
                                assert_eq!(
                                    completed.anchor_evidence(),
                                    OperationCustodyAnchorEvidence::Advanced
                                );
                                assert_eq!(
                                    port.observe_exact(completed.candidate()).unwrap(),
                                    OperationCustodyReadBackOutcome::ExactSuccessor
                                );
                                let retained_context: bool = port
                                    .connection
                                    .query_row(
                                        "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record WHERE record_ref=?1)",
                                        ["context/projection"],
                                        |row| row.get(0),
                                    )
                                    .unwrap();
                                assert!(
                                    !retained_context,
                                    "context ownership remains outside Pale custody"
                                );
                                let checkpoint_ref = completed
                                    .candidate()
                                    .checkpoint()
                                    .successor()
                                    .coordinate()
                                    .record_ref()
                                    .clone();
                                let checkpoint = read_checkpoint(
                                    completed.candidate().checkpoint().successor_record(),
                                    &fields,
                                );
                                let context_reference = prepared.observation().0.clone();
                                let context_observation = prepared.observation().1.clone();
                                let known = [
                                    (first_record.coordinate().record_ref(), &first),
                                    (&context_reference, &context_observation),
                                ];
                                let mut reopened = connection(file);
                                let mut reopened_port = SqliteOperationCustody {
                                    connection: &mut reopened,
                                    coordinate: coordinate.clone(),
                                };
                                let recovered = history::recover(
                                    history::Input {
                                        operation,
                                        coordinate: &coordinate,
                                        selected_member: (&context_reference, &context_observation),
                                        known: &known,
                                        checkpoint: &fields,
                                        signing: parents.signing,
                                    },
                                    &mut reopened_port,
                                    &mut anchor,
                                );
                                let binding = CompletedOperationContextBinding::recover(
                                    operation,
                                    &recovered,
                                )
                                .unwrap();
                                assert!(matches!(
                                    binding.observation().1.state(),
                                    OperationObservationStateV1::Active {
                                        phase: OperationActivePhaseV1::ContextBound {
                                            evidence: actual
                                        }
                                    } if actual == &evidence
                                ));
                                (binding, checkpoint_ref, checkpoint)
                            },
                        );
                    let observation = OperationObservationAdmissionContextV1 {
                        signed: initial_observation.signed,
                        handle_ref,
                        handle,
                        predecessor: Some(context_binding.as_ref().map_or(
                            (first_record.coordinate().record_ref(), &first),
                            |(binding, _, _)| binding.observation(),
                        )),
                    };
                    let checkpoint = context_binding.as_ref().map_or(
                        (
                            initial_candidate
                                .checkpoint()
                                .successor()
                                .coordinate()
                                .record_ref(),
                            &initial_checkpoint,
                        ),
                        |(_, reference, checkpoint)| (reference, checkpoint),
                    );
                    let attempt_id = CapabilityAttemptIdV1::parse("native-sqlite-attempt").unwrap();
                    let kind =
                        BoundedResidentTextV1::parse(CapabilityProvider::NativeRust.as_str())
                            .unwrap();
                    let version = BoundedResidentTextV1::parse(
                        registry
                            .contract(PATH_INVENTORY_ACTION_ID)
                            .unwrap()
                            .version
                            .to_string(),
                    )
                    .unwrap();
                    let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
                    let contracts = local_start_contract_set();
                    let request_binding = request.local_request_binding().unwrap();
                    let admission = CapabilitySuccessAdmissionContextV1 {
                        result: CapabilityResultAdmissionContextV1 {
                            expected_schema_sha256: &result_schema,
                            expected_request_binding: &request_binding,
                            expected_attempt_id: &attempt_id,
                            expected_implementation_kind: &kind,
                            expected_implementation_version: &version,
                            signing_authority: parents.signing,
                        },
                        contract_set_ref: request_binding.contract_set_ref(),
                        contract_set: &contracts,
                    };
                    let attempt_input = NativeOperationAttemptInput {
                        context: OperationAttemptContext {
                            operation,
                            request,
                            session: parents.session,
                            current: parents.current,
                            lease: parents.lease,
                            context_binding: context_binding
                                .as_ref()
                                .map(|(binding, _, _)| binding),
                            generation,
                            selection_ref: record_ref("native/selection"),
                            maximum_selection_bytes: 65_536,
                            observation_ref: record_ref("native/prepared"),
                            observation,
                        },
                        registry: &registry,
                        attempt_id: &attempt_id,
                        result_admission: admission,
                    };
                    if let Journey::HookResponse(case) = journey {
                        let saved = service::run_hook_response(
                            case,
                            attempt_input,
                            checkpoint,
                            &mut port,
                            &mut anchor,
                        );
                        if let Some(saved) = saved {
                            drop(port);
                            drop(sql);
                            service::recover_hook_response(
                                saved,
                                if case.is_expiry() {
                                    Some([generations.pop().unwrap(), generations.pop().unwrap()])
                                } else {
                                    None
                                },
                                file,
                                &location,
                                &coordinate,
                                operation,
                                request,
                                &parents,
                                &fields,
                                &mut anchor,
                            );
                        }
                        return;
                    }
                    if let Journey::Service(case) = journey {
                        let completed =
                            service::run(case, attempt_input, checkpoint, &mut port, &mut anchor);
                        if let Some(completed) = completed {
                            drop(port);
                            drop(sql);
                            let mut reopened = connection(file);
                            reopened.pragma_update(None, "query_only", true).unwrap();
                            let mut port = SqliteOperationCustody {
                                connection: &mut reopened,
                                coordinate: coordinate.clone(),
                            };
                            service::read_back(&completed, &mut port);
                        }
                        return;
                    }
                    if matches!(journey, Journey::RecoveredService) {
                        let completed = service::run(
                            service::Case::Success,
                            attempt_input,
                            checkpoint,
                            &mut port,
                            &mut anchor,
                        )
                        .expect("protected recovery can re-enter the native service");
                        drop(port);
                        drop(sql);
                        let mut reopened = connection(file);
                        reopened.pragma_update(None, "query_only", true).unwrap();
                        let mut port = SqliteOperationCustody {
                            connection: &mut reopened,
                            coordinate: coordinate.clone(),
                        };
                        service::read_back(&completed, &mut port);
                        return;
                    }
                    let preparation_signs = AtomicUsize::new(0);
                    let attempt = prepare_native_operation_attempt(
                        attempt_input,
                        &mut ProvingAuthority,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            preparation_signs.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                    );
                    let attempt = attempt.unwrap();
                    assert_eq!(preparation_signs.load(Ordering::SeqCst), 1);
                    let (physical, fields) = prepare_append(
                        attempt.prepare_append(&resolve).unwrap(),
                        operation,
                        checkpoint,
                        &mut port,
                        63,
                    );
                    let prepared_done = physical
                        .commit(&mut port, &mut anchor)
                        .unwrap()
                        .advance_anchor(&mut anchor)
                        .unwrap();
                    assert!(prepared_done
                        .candidate()
                        .records()
                        .iter()
                        .any(|record| matches!(
                            record,
                            OperationCustodyRecord::AttemptSelection(_)
                        )));
                    let prepared_checkpoint = read_checkpoint(
                        prepared_done.candidate().checkpoint().successor_record(),
                        &fields,
                    );
                    let current = parents.current;
                    if context.has_read_authority() {
                        assert_eq!(parents.lease.read_set(), &[record_ref("read-a")]);
                        let retained_read: bool = port.connection.query_row(
                            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record WHERE record_ref=?1)",
                            ["read-a"],
                            |row| row.get(0),
                        ).unwrap();
                        assert!(
                            !retained_read,
                            "unused read authority requires no retained context body"
                        );
                    }
                    if matches!(
                        journey,
                        Journey::ChildStartWins
                            | Journey::ChildServiceSuccess
                            | Journey::ParentCancellationWins
                            | Journey::ChildStartTransactionChanges
                    ) {
                        let evidence = child_start::run(
                            child_start::Fixture {
                                operation,
                                request,
                                parents: &parents,
                                generation,
                                checkpoint: &fields,
                                file,
                                registry: &registry,
                            },
                            attempt,
                            NativeOperationCancellationInput {
                                completed: &prepared_done,
                                current,
                                signing: observation.signed,
                                diagnostic_ref: record_ref("native/child-race-cancel-diagnostic"),
                                observation_ref: record_ref("native/child-race-cancelled"),
                                maximum_diagnostic_bytes: 65_536,
                                resolve_store: &resolve,
                            },
                            &prepared_checkpoint,
                            &mut port,
                            &mut anchor,
                            match journey {
                                Journey::ChildStartWins => child_start::Order::ChildFirst,
                                Journey::ChildServiceSuccess => child_start::Order::ChildService,
                                Journey::ParentCancellationWins => {
                                    child_start::Order::CancellationFirst
                                }
                                Journey::ChildStartTransactionChanges => {
                                    child_start::Order::TransactionChangesDuringParentRead
                                }
                                _ => unreachable!("selected child-start journey"),
                            },
                        );
                        drop(sql);
                        drop(recovered_operation);
                        drop(issued_operation);
                        child_start::recover(
                            evidence,
                            request,
                            &parents,
                            generation,
                            &location,
                            file,
                            &mut anchor,
                        );
                        return;
                    }
                    if matches!(
                        journey,
                        Journey::CancellationWins
                            | Journey::DispatchWins
                            | Journey::ReadAuthorityCancellationWins
                            | Journey::ReadAuthorityDispatchWins
                    ) {
                        let winner = cancellation::race(
                            attempt,
                            NativeOperationCancellationInput {
                                completed: &prepared_done,
                                current,
                                signing: observation.signed,
                                diagnostic_ref: record_ref("native/cancel-diagnostic"),
                                observation_ref: record_ref("native/cancelled"),
                                maximum_diagnostic_bytes: 65_536,
                                resolve_store: &resolve,
                            },
                            operation,
                            &prepared_checkpoint,
                            &mut port,
                            &mut anchor,
                            matches!(
                                journey,
                                Journey::CancellationWins | Journey::ReadAuthorityCancellationWins
                            ),
                        );
                        drop(sql);
                        let mut sql = connection(file);
                        let mut port = SqliteOperationCustody {
                            connection: &mut sql,
                            coordinate: coordinate.clone(),
                        };
                        let writes = port.connection.total_changes();
                        assert_eq!(
                            port.observe_exact(&winner).unwrap(),
                            OperationCustodyReadBackOutcome::ExactSuccessor
                        );
                        assert_eq!(
                            port.compare_and_commit(&winner).unwrap(),
                            OperationCustodyCommitOutcome::ExactReplay
                        );
                        for record in winner.records() {
                            assert_eq!(
                                port.read_operation_record(
                                    &coordinate,
                                    record.record().coordinate(),
                                    65_536
                                )
                                .unwrap()
                                .as_ref(),
                                Some(record.record())
                            );
                        }
                        assert_eq!(
                            port.connection.total_changes(),
                            writes,
                            "reopened winning cancellation or dispatch is read-only replay"
                        );
                        return;
                    }
                    if matches!(
                        journey,
                        Journey::ControlCancellationWins
                            | Journey::ControlDispatchWins
                            | Journey::ControlFailureAfterCancellation
                    ) {
                        let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                            &location,
                            coordinate.clone(),
                        )
                        .unwrap();
                        control_cancellation::race(
                            attempt,
                            &prepared_done,
                            operation,
                            request,
                            &parents,
                            observation.signed,
                            &prepared_checkpoint,
                            &fields,
                            &sessions,
                            &mut port,
                            &mut anchor,
                            matches!(journey, Journey::ControlCancellationWins),
                            matches!(journey, Journey::ControlFailureAfterCancellation),
                        );
                        return;
                    }
                    let preparation_history = (
                        attempt.observation().0.clone(),
                        attempt.observation().1.clone(),
                    );
                    let dispatch = attempt
                        .prepare_dispatch(
                            NativeOperationDispatchContext {
                                completed: &prepared_done,
                                current,
                                signing: observation.signed,
                                observation_ref: record_ref("native/dispatched"),
                                resolve_store: &resolve,
                            },
                            &mut ProvingAuthority,
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap();
                    let prior = (
                        prepared_done
                            .candidate()
                            .checkpoint()
                            .successor()
                            .coordinate()
                            .record_ref(),
                        &prepared_checkpoint,
                    );
                    let (physical, fields) = prepare_append(
                        dispatch.prepare_append(&resolve).unwrap(),
                        operation,
                        prior,
                        &mut port,
                        64,
                    );
                    // Prepare a second exact candidate before commitment, then observe
                    // its real replay classification; do not manufacture completion.
                    let repeated = replay.then(|| {
                        prepare_append(
                            dispatch.prepare_append(&resolve).unwrap(),
                            operation,
                            prior,
                            &mut port,
                            64,
                        )
                        .0
                    });
                    let fresh_done = physical
                        .commit(&mut port, &mut anchor)
                        .unwrap()
                        .advance_anchor(&mut anchor)
                        .unwrap();
                    let dispatch_done = if let Some(repeated) = repeated {
                        drop(fresh_done);
                        let replayed = repeated
                            .commit(&mut port, &mut anchor)
                            .unwrap()
                            .advance_anchor(&mut anchor)
                            .unwrap();
                        assert_eq!(
                            replayed.database_evidence(),
                            OperationCustodyDatabaseEvidence::ExactReplay
                        );
                        replayed
                    } else {
                        fresh_done
                    };
                    let dispatch_candidate = dispatch_done.candidate().clone();
                    let dispatch_checkpoint = read_checkpoint(
                        dispatch_candidate.checkpoint().successor_record(),
                        &fields,
                    );
                    drop(sql);
                    let mut sql = connection(file);
                    let writes = sql.total_changes();
                    let mut port = SqliteOperationCustody {
                        connection: &mut sql,
                        coordinate: coordinate.clone(),
                    };
                    assert_eq!(
                        port.observe_exact(&dispatch_candidate).unwrap(),
                        OperationCustodyReadBackOutcome::ExactSuccessor
                    );
                    let crash_recovered_operation =
                        matches!(journey, Journey::CrashRecovery).then(|| {
                            let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                                &location,
                                coordinate.clone(),
                            )
                            .unwrap();
                            let known = [
                                (first_record.coordinate().record_ref(), &first),
                                (&preparation_history.0, &preparation_history.1),
                                dispatch.observation(),
                            ];
                            history::recover_saved(
                                &history::Input {
                                    operation,
                                    coordinate: &coordinate,
                                    selected_member: dispatch.observation(),
                                    known: &known,
                                    checkpoint: &fields,
                                    signing: parents.signing,
                                },
                                request,
                                &parents,
                                &sessions,
                                &mut port,
                                &mut anchor,
                            )
                        });
                    let operation = crash_recovered_operation
                        .as_ref()
                        .map_or(operation, RecoveredLocalOperation::operation);
                    let request = crash_recovered_operation
                        .as_ref()
                        .map_or(request, RecoveredLocalOperation::request);
                    let clock = Clock(AtomicUsize::new(0));
                    if matches!(journey, Journey::A2aCustody | Journey::A2aStreamingRecovery) {
                        let known = [
                            (first_record.coordinate().record_ref(), &first),
                            (&preparation_history.0, &preparation_history.1),
                            dispatch.observation(),
                        ];
                        let candidate = a2a_custody::run(
                            if matches!(journey, Journey::A2aStreamingRecovery) {
                                a2a_custody::Transport::Streaming
                            } else {
                                a2a_custody::Transport::Unary
                            },
                            (generation, effective),
                            history::Input {
                                operation,
                                coordinate: &coordinate,
                                selected_member: dispatch.observation(),
                                known: &known,
                                checkpoint: &fields,
                                signing: parents.signing,
                            },
                            &dispatch_done,
                            observation.signed,
                            &mut port,
                            &mut anchor,
                            a2a_custody::Restart {
                                request,
                                parents: &parents,
                                generation,
                                location: &location,
                                file,
                            },
                        );
                        assert_eq!(clock.0.load(Ordering::SeqCst), 0);
                        drop(port);
                        drop(sql);
                        a2a_custody::read_back(candidate, file, &coordinate);
                        return;
                    }
                    let execution_input = || NativeOperationExecutionContext {
                        current,
                        resolve_store: &resolve,
                        clock: &clock,
                        maximum_record_bytes: 65_536,
                    };
                    let crash_selection = matches!(journey, Journey::CrashRecovery).then(|| {
                        let selected_record = prepared_done
                            .candidate()
                            .records()
                            .iter()
                            .find_map(|record| match record {
                                OperationCustodyRecord::AttemptSelection(record) => Some(record),
                                _ => None,
                            })
                            .unwrap();
                        port.read_operation_record(
                            &coordinate,
                            selected_record.coordinate(),
                            65_536,
                        )
                        .unwrap()
                        .unwrap()
                    });
                    if matches!(
                        journey,
                        Journey::RefusalCancellation | Journey::RefusalExpiry
                    ) {
                        let run = if matches!(journey, Journey::RefusalExpiry) {
                            refusal_expiry::run
                        } else {
                            refusal_cancellation::run
                        };
                        let evidence = run(
                            dispatch,
                            dispatch_done,
                            refusal_cancellation::Context {
                                execution: execution_input(),
                                signing: observation.signed,
                                operation,
                                checkpoint: &dispatch_checkpoint,
                            },
                            &mut port,
                            &mut anchor,
                        );
                        if matches!(journey, Journey::RefusalExpiry) {
                            assert_eq!(
                                clock.0.load(Ordering::SeqCst),
                                0,
                                "expiry recording cannot reopen the original call or resample its closed clock"
                            );
                        }
                        drop(port);
                        drop(sql);
                        refusal_cancellation::read_back(evidence, file, &coordinate);
                        return;
                    }
                    let executed = if matches!(journey, Journey::CrashRecovery) {
                        let known = [
                            (first_record.coordinate().record_ref(), &first),
                            (&preparation_history.0, &preparation_history.1),
                            dispatch.observation(),
                        ];
                        let recovered = history::recover(
                            history::Input {
                                operation,
                                coordinate: &coordinate,
                                selected_member: dispatch.observation(),
                                known: &known,
                                checkpoint: &fields,
                                signing: parents.signing,
                            },
                            &mut port,
                            &mut anchor,
                        );
                        let recovered_dispatch = recover_native_operation_dispatch(
                            OperationAttemptHistoryInput {
                                operation,
                                request,
                                session: operation.historical_session_lease(),
                                lease: operation.historical_operation_lease(),
                                generation,
                                dispatched: dispatch.observation().1,
                                selection: crash_selection.as_ref().unwrap(),
                                maximum_selection_bytes: 65_536,
                            },
                            &recovered,
                            &registry,
                            &contracts,
                        )
                        .unwrap();
                        Ok(recovered_dispatch
                            .recompute(
                                NativeOperationRecoveryContext {
                                    current,
                                    clock: &clock,
                                    maximum_record_bytes: 65_536,
                                },
                                &mut ProvingAuthority,
                                &mut port,
                                &mut anchor,
                            )
                            .expect("the exact recovered pure dispatch is recomputable"))
                    } else if matches!(journey, Journey::BoundedReadRefusal) {
                        let probe_clock = Clock(AtomicUsize::new(0));
                        let mut bounded = execution_input();
                        bounded.clock = &probe_clock;
                        bounded.maximum_record_bytes = 1;
                        let refused = dispatch
                            .invoke(
                                dispatch_done,
                                bounded,
                                &mut ProvingAuthority,
                                &mut port,
                                &mut anchor,
                            )
                            .expect_err("bounded native read refuses before the handler");
                        assert!(matches!(
                            refused.reason(),
                            OperationAttemptError::Custody(_)
                        ));
                        assert_eq!(probe_clock.0.load(Ordering::SeqCst), 1);
                        assert_eq!(port.connection.total_changes(), writes);
                        // The original New confirmation remains inside the refusal.
                        // No recommit, replay or new completion is manufactured.
                        refused.recheck(
                            execution_input(),
                            &mut ProvingAuthority,
                            &mut port,
                            &mut anchor,
                        )
                    } else {
                        dispatch.invoke(
                            dispatch_done,
                            execution_input(),
                            &mut ProvingAuthority,
                            &mut port,
                            &mut anchor,
                        )
                    };
                    assert_eq!(
                        port.connection.total_changes(),
                        writes,
                        "release checks are read-only"
                    );
                    if replay {
                        assert!(matches!(
                            executed.as_ref().err().unwrap().reason(),
                            OperationAttemptError::Custody(
                                OperationDispatchCustodyError::NotNewCommit
                            )
                        ));
                        assert_eq!(clock.0.load(Ordering::SeqCst), 1);
                        return;
                    }
                    let outcome = executed.unwrap();
                    let dispatch_history_owned = match outcome.outcome() {
                        NativeOperationOutcome::Succeeded(value) => {
                            (value.dispatch().0.clone(), value.dispatch().1.clone())
                        }
                        NativeOperationOutcome::Failed(value) => {
                            (value.dispatch().0.clone(), value.dispatch().1.clone())
                        }
                    };
                    drop(port);
                    let fixture = OutcomeFixture {
                        journey,
                        generation,
                        operation,
                        request,
                        parents,
                        coordinate: coordinate.clone(),
                        location: &location,
                        file,
                        first_record,
                        first: &first,
                        prepared_done: &prepared_done,
                        preparation_history,
                        dispatch_candidate,
                        dispatch_checkpoint,
                        dispatch_history_owned,
                        observation,
                        registry: &registry,
                        contracts: &contracts,
                        clock: &clock,
                        anchor,
                    };
                    if matches!(
                        journey,
                        Journey::StagedRecoverySuccess
                            | Journey::StagedRecoveryFailure
                            | Journey::StagedRecoveryAfterCancellation
                            | Journey::StagedOutcomeSubstitution
                            | Journey::StagedFailureDiagnosticSubstitution
                            | Journey::StagedFailureRequestSubstitution
                            | Journey::StagedFailureRequestKindSubstitution
                            | Journey::StagedFailureRequestMissing
                            | Journey::StagedOutcomeMissingIndex
                            | Journey::StagedOutcomeMissingEvidence
                            | Journey::StagedOutcomeMalformedEvidence
                            | Journey::StagedOutcomeWrongSchema
                    ) {
                        staged::run(fixture, outcome, sql);
                        return;
                    }
                    terminal::run(
                        fixture,
                        outcome,
                        sql,
                        context_binding
                            .as_ref()
                            .map(|(binding, _, _)| binding.observation()),
                        crash_selection,
                    );
                },
            );
        }
    });
}
