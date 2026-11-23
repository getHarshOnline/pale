//! A second real operation domain predates physical route custody in the first.

//! Contextual dispatch evidence remains a fixture model, not a network send claim.

use super::*;

pub(super) struct Root {
    operation: IssuedLocalOperation,
    first_ref: ResidentRecordRefV1,
    first: OperationObservationV1,
    dispatch_ref: ResidentRecordRefV1,
    dispatch: OperationObservationV1,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint: ControlStateCheckpointV1,
    fields: ControlStateCheckpointPreimageV1,
    signing: SigningAuthorityBindingV1,
    anchor: IndependentAnchor,
}

pub(super) fn start(
    generation: &AdmittedCompiledGeneration,
    effective: &EffectiveProjectProfileV1,
    port: &mut SqliteOperationCustody<'_>,
) -> Root {
    with_start_context(
        generation,
        effective,
        "a2a-second-root",
        native_request_body,
        StartAdmission::AcceptedOwnerModel,
        LocalStartContext::SelfContained,
        |prepared, request, owners, signs, parents| {
            assert_eq!((owners, signs), (1, 2));
            let coordinate = port.coordinate.clone();
            retain_operation_lease_body(
                port,
                &coordinate,
                &record_ref("start-lease"),
                parents.lease,
                1 << 20,
            )
            .unwrap();
            let prepared = prepared
                .unwrap()
                .prepare_checkpoint(
                    AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000).unwrap(),
                    port,
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
            let first_checkpoint_ref = record_ref("a2a-second/checkpoint-1");
            let (physical, pending) = prepared
                .issue_checkpoint(
                    &first_checkpoint_ref,
                    fields.clone(),
                    &fields.schema_sha256,
                    &checkpoint_authority(&fields),
                    |_, _, _| Ok(signature()),
                )
                .unwrap();
            let mut anchor = IndependentAnchor::default();
            let completed = physical
                .commit(port, &mut anchor, 1 << 20)
                .unwrap()
                .advance_anchor(&mut anchor)
                .unwrap();
            let checkpoint = read_checkpoint(
                completed.candidate().checkpoint().successor_record(),
                &fields,
            );
            let operation = pending
                .finish(completed)
                .unwrap_or_else(|_| panic!("second root custody"));
            let first_ref = record_ref("observation/a2a-second-root");
            let first = issue_observation(
                parents.signing,
                operation.handle().0,
                operation.handle().1,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            let dispatch_ref = record_ref("a2a-second/dispatched");
            let dispatch = issue_observation(
                parents.signing,
                operation.handle().0,
                operation.handle().1,
                first.schema_sha256().clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&first_ref, &first)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched {
                        attempt_id: CapabilityAttemptIdV1::parse("a2a-second-attempt").unwrap(),
                        evidence: fixture::evidence(
                            "a2a-second/dispatch-model",
                            b"explicit fixture dispatch authority",
                        ),
                    },
                },
                64,
            );
            let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
            let append = prepare_operation_observation_append(
                OperationObservationTransition::new(
                    operation.handle().0,
                    operation.handle().1,
                    &first_ref,
                    &first,
                    &dispatch_ref,
                    &dispatch,
                ),
                &resolve,
            )
            .unwrap();
            let checkpoint_ref = record_ref("a2a-second/checkpoint-2");
            let (prepared, fields) = prepare_append_with_ref(
                append,
                &operation,
                (&first_checkpoint_ref, &checkpoint),
                port,
                64,
                &checkpoint_ref,
            );
            let completed = prepared
                .commit(port, &mut anchor)
                .unwrap()
                .advance_anchor(&mut anchor)
                .unwrap();
            let checkpoint = read_checkpoint(
                completed.candidate().checkpoint().successor_record(),
                &fields,
            );
            Root {
                operation,
                first_ref,
                first,
                dispatch_ref,
                dispatch,
                checkpoint_ref,
                checkpoint,
                fields,
                signing: parents.signing.clone(),
                anchor,
            }
        },
    )
}

impl Root {
    pub(super) fn associate(
        &mut self,
        route: &A2aRouteAdmissionV1,
        first_domain: &OperationCustodyCandidate,
        port: &mut SqliteOperationCustody<'_>,
    ) -> OperationCustodyCandidate {
        let record = first_domain
            .record(OperationCustodyRecordKind::A2aRouteAdmission)
            .unwrap();
        let route_stamp: i64 = port.connection.query_row("SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=?1", [record.coordinate().record_ref().as_str()], |row| row.get(0)).unwrap();
        assert!(
            u64::try_from(route_stamp).unwrap()
                > self.checkpoint.committed_transaction_sequence().get(),
            "shared route was physically stored AFTER this root's checkpoint"
        );
        assert_ne!(
            first_domain.checkpoint().successor().checkpoint_domain_id(),
            self.checkpoint.checkpoint_domain_id()
        );
        let original_local = current_local(port, &self.operation);
        let coordinate = port.coordinate.clone();
        let proof = history::recover(
            history::Input {
                operation: &self.operation,
                coordinate: &coordinate,
                selected_member: (&self.dispatch_ref, &self.dispatch),
                known: &[
                    (&self.first_ref, &self.first),
                    (&self.dispatch_ref, &self.dispatch),
                ],
                checkpoint: &self.fields,
                signing: &self.signing,
            },
            port,
            &mut self.anchor,
        );
        let request = raw("a2a-second/request", b"second fixture original request");
        let response = raw("a2a-second/response", b"second fixture Task response");
        let mut inputs = fixture::Inputs::new(
            self.operation.handle(),
            (&self.dispatch_ref, &self.dispatch),
            (record.coordinate().record_ref(), route),
            fixture::evidence("a2a-second/request", request.bytes()),
            fixture::evidence("a2a-second/response", response.bytes()),
            100,
        );
        inputs.binding_ref = record_ref("a2a-second/binding");
        inputs.binding.task_binding_id = A2aTaskBindingIdV1::parse("a2a-second-binding").unwrap();
        inputs.observation_ref = record_ref("a2a-second/observation");
        inputs.observation.task_binding_ref = inputs.binding_ref.clone();
        inputs.observation.task_binding_id = inputs.binding.task_binding_id.clone();
        inputs.observation.task_binding_sha256 =
            A2aTaskBindingDigest::hash_canonical(&inputs.binding).unwrap();
        let append = prepare_a2a_task_binding_custody(
            A2aTaskBindingCustodyInput {
                operation_custody: &proof,
                binding_ref: inputs.binding_ref.clone(),
                binding: inputs.binding.clone(),
                binding_context: inputs.binding_context(
                    self.operation.handle(),
                    (&self.dispatch_ref, &self.dispatch),
                    route,
                    &self.signing,
                ),
                observation_ref: inputs.observation_ref.clone(),
                observation: inputs.observation.clone(),
                observation_signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &inputs.observation.schema_sha256,
                    signing_authority: &self.signing,
                },
                route_predecessor: OperationCustodyIndexPredecessor::Exact(record.clone()),
                route_association: A2aRouteAssociation::Insert,
                new_evidence: vec![request, response],
                retained_evidence: vec![],
                maximum_evidence_bytes: 8192,
                maximum_record_bytes: 65_536,
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
            |_, _, _| Ok(signature()),
            |_| Ok(coordinate.clone()),
        )
        .unwrap();
        let (prepared, _) = physical(
            append,
            &self.operation,
            (&self.checkpoint_ref, &self.checkpoint),
            port,
            100,
            "second-root-association",
        )
        .unwrap();
        let completed = prepared
            .commit(port, &mut self.anchor)
            .unwrap()
            .advance_anchor(&mut self.anchor)
            .unwrap();
        assert_exact(port, completed.candidate());
        assert!(
            completed
                .candidate()
                .record(OperationCustodyRecordKind::A2aRouteAdmission)
                .is_none(),
            "the shared signed route is not inserted again"
        );
        assert_eq!(completed.candidate().a2a_associated_route(), Some(record));
        assert_eq!(
            port.read_operation_record(&coordinate, record.coordinate(), 65_536)
                .unwrap()
                .as_ref(),
            Some(record)
        );
        let unchanged_stamp: i64 = port.connection.query_row("SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=?1", [record.coordinate().record_ref().as_str()], |row| row.get(0)).unwrap();
        assert_eq!(unchanged_stamp, route_stamp);
        assert_eq!(current_local(port, &self.operation), original_local);
        completed.candidate().clone()
    }
}
