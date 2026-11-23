//! Restarted discovery and selected recovery over actual SQLite-backed custody. Historical
//! route, signature and other exchange meaning remain explicit contextual
//! fixtures. `GetTask` protocol readmission proves no network send or current grant.

use super::*;
use crate::location::ControlStoreLocation;
use crate::resident_fixture::local_operation_start_fixture::{
    profile, publication_writer, RecoveryParents,
};
use crate::resident_fixture::producer_custody_fixture::InitialProducerFixture;
use crate::resident_fixture::shared_resident::{FixedInputSource, RecoveryResolver};
use pale_core::canon;
use pale_kernel::AdaptiveNodeSetPolicy;
use std::collections::BTreeSet;

pub(in super::super) struct Restart<'a> {
    pub(in super::super) request: &'a CommonRequestEnvelopeV1,
    pub(in super::super) parents: &'a RecoveryParents<'a>,
    pub(in super::super) generation: &'a AdmittedCompiledGeneration,
    pub(in super::super) location: &'a ControlStoreLocation,
    pub(in super::super) file: &'a std::path::Path,
}

struct Expected<'a> {
    local: history::Input<'a>,
    inputs: &'a fixture::Inputs,
    binding: &'a A2aTaskBindingV1,
    route: &'a A2aRouteAdmissionV1,
    head: (&'a ResidentRecordRefV1, &'a A2aTaskObservationV1),
}

fn verify_checkpoint(
    domain: &[u8],
    _: &ControlStateCheckpointPreimageV1,
    _: &ControlStateCheckpointDigest,
    actual: &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1> {
    assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
    assert_eq!(actual, &signature());
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum Limit {
    Exact,
    Revisions,
    RecordBytes,
    HistoryBytes,
    EvidenceRecords,
    EvidenceBytes,
    LocalRevisions,
}

fn recover(
    expected: &Expected<'_>,
    operation: &IssuedLocalOperation,
    binding_id: &A2aTaskBindingIdV1,
    discovered: Option<&DiscoveredA2aTaskBinding>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
    limit: Limit,
) -> Result<RecoveredA2aTaskHistory, A2aTaskRecoveryError> {
    recover_selected(
        expected,
        operation,
        binding_id,
        discovered,
        port,
        anchor,
        limit,
        A2aTaskObservationSelection::SelectedHead,
    )
}

#[allow(clippy::too_many_arguments)]
fn recover_selected(
    expected: &Expected<'_>,
    operation: &IssuedLocalOperation,
    binding_id: &A2aTaskBindingIdV1,
    discovered: Option<&DiscoveredA2aTaskBinding>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
    limit: Limit,
    observation: A2aTaskObservationSelection<'_>,
) -> Result<RecoveredA2aTaskHistory, A2aTaskRecoveryError> {
    let local = &expected.local;
    let reads = Cell::new(0);
    let states = Cell::new(0);
    let remote_reads = Cell::new(0);
    let binding_reads = Cell::new(0);
    let route_reads = Cell::new(0);
    let evidence_bytes: usize = port.connection.query_row(
        "SELECT sum(length(evidence_bytes)+coalesce(length(sse_event_id),0)+coalesce(length(sse_event_type),0)) FROM pale_deployment_operation_a2a_evidence WHERE record_ref LIKE 'a2a/%'",
        [], |row| row.get::<_, i64>(0)).map(|n| usize::try_from(n).unwrap()).unwrap();
    let admission = OperationObservationRecoveryAdmission {
        admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
            ControlStateCheckpointV1::admit_signed_wire(
                wire,
                &local.checkpoint.schema_sha256,
                &checkpoint_authority(local.checkpoint),
                verify_checkpoint,
            )
        },
        resolve_signer: |reference: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
            assert!(local.known.iter().any(|(known, _)| *known == reference));
            reads.set(reads.get() + 1);
            Ok(local.signing.clone())
        },
        admit_state: |state: &OperationObservationStateV1| {
            assert!(local
                .known
                .iter()
                .any(|(_, record)| record.state() == state));
            states.set(states.get() + 1);
            Ok(())
        },
        admit_rotation: |_: &ResidentRecordRefV1,
                         _: &ArtifactDigest,
                         _: &ResidentRecordRefV1,
                         _: &ArtifactDigest| {
            panic!("no authority rotation in this fixture")
        },
        verify_signature: |domain: &[u8],
                           _: &OperationObservationPreimageV1,
                           _: &OperationObservationDigest,
                           actual: &CanonicalSignatureV1| {
            assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
    };
    let mut route = |reference: &ResidentRecordRefV1, wire: A2aRouteAdmissionWireV1| {
        route_reads.set(route_reads.get() + 1);
        assert_eq!(reference, &expected.inputs.route_ref);
        A2aRouteAdmissionV1::admit_contextual_wire(
            wire,
            &A2aRouteAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: expected.route.schema_sha256(),
                    signing_authority: local.signing,
                },
                provider_binding: expected.route.provider_binding(),
                route: expected.route.route(),
            },
            |preimage| {
                assert_eq!(
                    &A2aRouteAdmissionDigest::hash_canonical(preimage).unwrap(),
                    expected.route.canonical_a2a_route_admission_sha256()
                );
                Ok(()) // Explicit historical Agent Card/security/HTTPS authority model.
            },
            |domain, _, _, actual| {
                assert_eq!(domain, A2A_ROUTE_ADMISSION_V1_SIGNATURE_DOMAIN);
                assert_eq!(actual, &signature());
                Ok(())
            },
        )
    };
    let mut binding_signer = |reference: &ResidentRecordRefV1, _: &A2aTaskBindingWireV1| {
        assert_eq!(reference, &expected.inputs.binding_ref);
        binding_reads.set(binding_reads.get() + 1);
        Ok(local.signing.clone())
    };
    let mut binding_evidence = |preimage: &A2aTaskBindingPreimageV1,
                                evidence: &[&OperationA2aEvidence]| {
        assert_eq!(preimage, &expected.inputs.binding);
        assert_eq!(evidence.len(), 2);
        let request: &[u8] = match expected.binding.request_mode() {
            A2aTaskRequestModeV1::SendMessage => {
                b"{ \"id\":\"rpc-1\", \"method\":\"SendMessage\" }\n"
            }
            A2aTaskRequestModeV1::SendStreamingMessage => {
                b"{ \"id\":\"rpc-1\", \"method\":\"SendStreamingMessage\" }\n"
            }
        };
        assert_eq!(evidence[0].bytes(), request);
        assert_eq!(evidence[1].bytes(), b"{ \"task\": \"fixture-task-1\" }\n");
        Ok(()) // Contextual exchange meaning is fixture authority, not transport evidence.
    };
    let mut binding_signature = |domain: &[u8],
                                 _: &A2aTaskBindingPreimageV1,
                                 _: &A2aTaskBindingDigest,
                                 actual: &CanonicalSignatureV1| {
        assert_eq!(domain, A2A_TASK_BINDING_V1_SIGNATURE_DOMAIN);
        assert_eq!(actual, &signature());
        Ok(())
    };
    let mut observation_signer = |_: &ResidentRecordRefV1, _: &A2aTaskObservationWireV1| {
        remote_reads.set(remote_reads.get() + 1);
        Ok(local.signing.clone())
    };
    let mut observation_evidence =
        |preimage: &A2aTaskObservationPreimageV1, evidence: &[&OperationA2aEvidence]| {
            assert_eq!(
                preimage.task_binding_id,
                *expected.binding.task_binding_id()
            );
            assert!(!evidence.is_empty());
            for value in evidence {
                assert_eq!(
                    ArtifactDigest::hash_bytes(value.bytes()).as_str(),
                    value.coordinate().record_sha256().as_str()
                );
            }
            match &preimage.source {
                A2aTaskObservationSourceV1::GetTask { .. } => {
                    get_task::admit(
                        (&expected.inputs.binding_ref, expected.binding),
                        preimage,
                        evidence,
                    )?;
                }
                A2aTaskObservationSourceV1::InitialStreamingEvent { event_id, .. }
                | A2aTaskObservationSourceV1::StreamingEvent { event_id, .. } => {
                    assert_eq!(evidence.len(), 1);
                    if evidence[0].sse() != Some((event_id.as_ref().unwrap().as_str(), "message")) {
                        return Err(InvalidResidentRecordV1::ContextualAdmission {
                            record: "A2A observation fixture",
                            invariant: "exact event metadata",
                        });
                    }
                }
                _ => {}
            }
            if let A2aTaskObservationSourceV1::CancelTaskReadBack { request, .. } = &preimage.source
            {
                assert_eq!(
                    request,
                    &fixture::evidence(
                        "a2a/cancel-request",
                        b"{ \"method\":\"CancelTask\", \"id\":\"cancel-1\" }\n"
                    )
                );
                assert_eq!(
                    &A2aTaskObservationDigest::hash_canonical(preimage).unwrap(),
                    expected.head.1.canonical_a2a_task_observation_sha256()
                );
            }
            // Non-GetTask sources retain their explicit fixture admission above.
            // Neither protocol qualification nor this model asserts a local result.
            Ok(())
        };
    let mut rotation = |_: &ResidentRecordRefV1,
                        _: &ArtifactDigest,
                        _: &ResidentRecordRefV1,
                        _: &ArtifactDigest|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("no remote authority rotation in this fixture")
    };
    let mut observation_signature =
        |domain: &[u8],
         _: &A2aTaskObservationPreimageV1,
         _: &A2aTaskObservationDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(domain, A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        };
    let limits = A2aTaskRecoveryLimits::new(
        if matches!(limit, Limit::Revisions) {
            3
        } else {
            4
        },
        if matches!(limit, Limit::RecordBytes) {
            1
        } else {
            65_536
        },
        if matches!(limit, Limit::HistoryBytes) {
            1
        } else {
            1 << 20
        },
        if matches!(limit, Limit::EvidenceRecords) {
            6
        } else {
            7
        },
        if matches!(limit, Limit::EvidenceBytes) {
            evidence_bytes - 1
        } else {
            evidence_bytes
        },
    )
    .unwrap();
    let checkpoint_signer = checkpoint_authority(local.checkpoint);
    let input = A2aTaskRecoveryInput {
        store: local.coordinate,
        operation,
        task_binding_id: binding_id,
        observation,
        checkpoint_authority: &checkpoint_signer,
        local_limits: OperationObservationRecoveryLimits::new(
            if matches!(limit, Limit::LocalRevisions) {
                local.known.len() - 1
            } else {
                local.known.len()
            },
            1 << 20,
            65_536,
        )
        .unwrap(),
        checkpoint_limits: OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
        membership_limits: AccumulatorMembershipLimits::new(32, 100_000, 1_000_000).unwrap(),
        limits,
    };
    let remote = A2aTaskRecoveryAdmission {
        admit_route: &mut route,
        resolve_binding_signer: &mut binding_signer,
        admit_binding_evidence: &mut binding_evidence,
        verify_binding_signature: &mut binding_signature,
        resolve_observation_signer: &mut observation_signer,
        admit_observation_evidence: &mut observation_evidence,
        admit_rotation: &mut rotation,
        verify_observation_signature: &mut observation_signature,
    };
    let result = match discovered {
        Some(selected) => {
            selected.recover(input, port, anchor, admission, verify_checkpoint, remote)
        }
        None => recover_a2a_task_history(input, port, anchor, admission, verify_checkpoint, remote),
    };
    assert!(
        reads.get() <= local.known.len(),
        "no per-remote-revision local rescans"
    );
    assert!(states.get() <= local.known.len());
    if matches!(limit, Limit::LocalRevisions) {
        assert!(
            matches!(&result, Err(A2aTaskRecoveryError::Local(_))),
            "the local revision bound must reach the local recovery fence"
        );
    }
    if matches!(&result, Err(A2aTaskRecoveryError::Local(_))) {
        assert_eq!(
            (route_reads.get(), binding_reads.get(), remote_reads.get()),
            (0, 0, 0),
            "remote owners cannot run before local anchored recovery succeeds"
        );
    }
    if result.is_ok() {
        assert_eq!(
            (reads.get(), states.get()),
            (local.known.len(), local.known.len())
        );
        assert_eq!(
            (route_reads.get(), binding_reads.get(), remote_reads.get()),
            (1, 1, 4)
        );
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    restart: Restart<'_>,
    local: history::Input<'_>,
    inputs: &fixture::Inputs,
    binding: &A2aTaskBindingV1,
    route: &A2aRouteAdmissionV1,
    head: (&ResidentRecordRefV1, &A2aTaskObservationV1),
    candidate: &OperationCustodyCandidate,
    anchor: &mut IndependentAnchor,
) {
    // New connections and protected local recovery, not an in-memory handle cast.
    let mut sql = connection(restart.file);
    let mut port = SqliteOperationCustody {
        connection: &mut sql,
        coordinate: local.coordinate.clone(),
    };
    let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
        restart.location,
        local.coordinate.clone(),
    )
    .unwrap();
    let mut resolver = RecoveryResolver::default();
    let generation = &restart.parents.current.generation;
    resolver.insert_generation(generation.compiled_generation);
    resolver.insert_active(generation.active_head);
    resolver.insert_fence(generation.activation_fence);
    resolver.insert_effective_profile(generation.effective_profile);
    let supported = profile();
    anchor.require_read_only();
    let operation = recover_local_operation(
        LocalOperationRecoveryContext {
            store: local.coordinate,
            profile: &supported,
            handle_signing_authority: local.signing,
            checkpoint_authority: &checkpoint_authority(local.checkpoint),
            historical: &resolver,
            maximum_record_bytes: 65_536,
            maximum_operation_lease_bytes: 65_536,
            maximum_session_lease_bytes: 65_536,
            membership_limits: AccumulatorMembershipLimits::new(8, 100_000, 1_000_000).unwrap(),
            observation_limits: OperationObservationRecoveryLimits::new(8, 65_536, 65_536).unwrap(),
            checkpoint_limits: OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
        },
        restart.request,
        &mut port,
        &sessions,
        anchor,
        LocalOperationRecoveryAdmission {
            admit_historical_authority:
                |actual: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                    assert!(parent.is_none());
                    assert_eq!(
                        &OperationHandleDigest::hash_canonical(actual).unwrap(),
                        local
                            .operation
                            .handle()
                            .1
                            .canonical_operation_handle_sha256()
                    );
                    Ok(())
                },
            verify_handle_signature:
                |domain: &[u8],
                 _: &OperationHandlePreimageV1,
                 _: &OperationHandleDigest,
                 actual: &CanonicalSignatureV1| {
                    assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            verify_checkpoint_signature: verify_checkpoint,
            initial_observation: OperationObservationRecoveryAdmission {
                admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                    ControlStateCheckpointV1::admit_signed_wire(
                        wire,
                        &local.checkpoint.schema_sha256,
                        &checkpoint_authority(local.checkpoint),
                        verify_checkpoint,
                    )
                },
                resolve_signer: |reference: &ResidentRecordRefV1,
                                 _: &OperationObservationWireV1| {
                    assert!(local.known.iter().any(|(known, _)| *known == reference));
                    Ok(local.signing.clone())
                },
                admit_state: |state: &OperationObservationStateV1| {
                    assert!(local.known.iter().any(|(_, known)| known.state() == state));
                    Ok(())
                },
                admit_rotation: |_: &ResidentRecordRefV1,
                                 _: &ArtifactDigest,
                                 _: &ResidentRecordRefV1,
                                 _: &ArtifactDigest| {
                    panic!("A2A recovery fixture has no local authority rotation")
                },
                verify_signature:
                    |domain: &[u8],
                     _: &OperationObservationPreimageV1,
                     _: &OperationObservationDigest,
                     actual: &CanonicalSignatureV1| {
                        assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        Ok(())
                    },
            },
        },
    )
    .unwrap();
    assert_eq!(operation.handle(), local.operation.handle());
    port.connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let discovered =
        discover_a2a_task_bindings(local.coordinate, &operation, None, 128, 1 << 20, &mut port)
            .unwrap();
    assert_eq!(discovered.bindings.len(), 3);
    assert!(discovered.continuation.is_none());
    let selected = &discovered.bindings[0];
    let expected = Expected {
        local,
        inputs,
        binding,
        route,
        head,
    };
    let before = current_local(&mut port, &operation);
    let writes = port.connection.total_changes();
    let recovered = recover(
        &expected,
        &operation,
        selected.task_binding_id(),
        Some(selected),
        &mut port,
        anchor,
        Limit::Exact,
    )
    .unwrap();
    assert_eq!(recovered.route(), (&inputs.route_ref, route));
    assert_eq!(recovered.binding(), (&inputs.binding_ref, binding));
    assert_eq!(selected.coordinate().record_ref(), recovered.binding().0);
    assert_eq!(
        selected.coordinate().record_sha256().as_str(),
        recovered
            .binding()
            .1
            .canonical_a2a_task_binding_sha256()
            .as_str()
    );
    assert_eq!(recovered.observation(), head);
    assert_eq!(recovered.selected_observation(), head);
    assert_eq!(recovered.dispatch(), expected.local.selected_member);
    assert_eq!(
        recovered.local_observation().selected_head(),
        *expected.local.known.last().unwrap()
    );
    assert!(matches!(
        recovered.local_observation().selected_head().1.state(),
        OperationObservationStateV1::OutcomeUnknown { .. }
    ));
    assert_eq!(head.1.remote_state(), &A2aRemoteTaskStateV1::Completed);
    let initial_digest = A2aTaskObservationDigest::hash_canonical(&inputs.observation).unwrap();
    let historical = recover_selected(
        &expected,
        &operation,
        selected.task_binding_id(),
        Some(selected),
        &mut port,
        anchor,
        Limit::Exact,
        A2aTaskObservationSelection::HistoryMember {
            reference: &inputs.observation_ref,
            digest: &initial_digest,
        },
    )
    .unwrap();
    assert_eq!(historical.observation(), head);
    assert_eq!(historical.selected_observation().0, &inputs.observation_ref);
    assert_eq!(
        historical
            .selected_observation()
            .1
            .canonical_a2a_task_observation_sha256(),
        &initial_digest
    );
    let publication_source = publication::all(&mut port)
        .into_iter()
        .find(|source| source.observation().coordinate().record_ref() == &inputs.observation_ref)
        .expect("initial A2A observation publication source");
    let checkpoint_schema =
        pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID).unwrap();
    let checkpoint_wire: ControlStateCheckpointWireV1 = serde_json::from_value(
        canon::parse_strict(
            publication_source
                .checkpoint()
                .canonical_record_jcs()
                .as_slice(),
        )
        .unwrap(),
    )
    .unwrap();
    let checkpoint = ControlStateCheckpointV1::admit_signed_wire(
        checkpoint_wire,
        &checkpoint_schema.schema_sha256,
        &checkpoint_authority(expected.local.checkpoint),
        verify_checkpoint,
    )
    .unwrap();
    let references = A2aTaskPublicationReferences::derive(
        expected.local.coordinate,
        historical.selected_observation().0,
        historical.selected_observation().1,
    )
    .unwrap();
    let replay_references = A2aTaskPublicationReferences::derive(
        expected.local.coordinate,
        historical.selected_observation().0,
        historical.selected_observation().1,
    )
    .unwrap();
    assert_eq!(references, replay_references);
    let _plan = admit_a2a_task_publication_plan(
        expected.local.coordinate,
        &publication_source,
        &historical,
        &operation,
        (
            publication_source.checkpoint().coordinate().record_ref(),
            &checkpoint,
        ),
        &references,
        None,
    )
    .unwrap();
    let unlike_digest = A2aTaskObservationDigest::parse("f".repeat(64)).unwrap();
    assert!(matches!(
        recover_selected(
            &expected,
            &operation,
            selected.task_binding_id(),
            Some(selected),
            &mut port,
            anchor,
            Limit::Exact,
            A2aTaskObservationSelection::HistoryMember {
                reference: &inputs.observation_ref,
                digest: &unlike_digest,
            },
        ),
        Err(A2aTaskRecoveryError::Missing)
    ));
    current_read_authorization_precedes_private_selection(
        restart.request,
        restart.generation,
        &operation,
        &recovered,
        &mut port,
    );
    for limit in [
        Limit::Revisions,
        Limit::RecordBytes,
        Limit::HistoryBytes,
        Limit::EvidenceRecords,
        Limit::EvidenceBytes,
        Limit::LocalRevisions,
    ] {
        assert!(
            recover(
                &expected,
                &operation,
                binding.task_binding_id(),
                None,
                &mut port,
                anchor,
                limit
            )
            .is_err(),
            "{limit:?}"
        );
    }
    let unlike = A2aTaskBindingIdV1::parse("unlike-task-binding").unwrap();
    assert!(recover(
        &expected,
        &operation,
        &unlike,
        None,
        &mut port,
        anchor,
        Limit::Exact
    )
    .is_err());
    assert_eq!(port.connection.total_changes(), writes);
    assert_eq!(current_local(&mut port, &operation), before);
    let OperationCheckpointPhase::Successor { predecessor, .. } = candidate.checkpoint().phase()
    else {
        panic!("remote history has a predecessor checkpoint");
    };
    let stale = AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
        expected.local.coordinate.store_id().clone(),
        expected.local.coordinate.incarnation_id().clone(),
        predecessor.checkpoint_domain_id().clone(),
        predecessor.domain_epoch(),
        predecessor.domain_sequence(),
        predecessor.coordinate().clone(),
    ));
    for switch_after in [1, 2] {
        let mut changed = ChangedAnchor {
            current: anchor,
            stale: stale.clone(),
            reads: 0,
            switch_after,
        };
        let rejected = recover(
            &expected,
            &operation,
            binding.task_binding_id(),
            None,
            &mut port,
            &mut changed,
            Limit::Exact,
        );
        assert!(rejected.is_err());
        if switch_after == 1 {
            assert!(
                matches!(rejected, Err(A2aTaskRecoveryError::Local(_))),
                "the first changed-anchor case must fail inside local recovery"
            );
        }
        assert!(
            changed.reads > switch_after,
            "reach the intended later independent-anchor fence"
        );
    }
    port.connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    corruptions(&expected, &operation, &mut port, anchor, candidate);
    assert_exact(&mut port, candidate);
    publish_remote_history(&restart, &expected, &operation, anchor);
}

fn publish_remote_history(
    restart: &Restart<'_>,
    expected: &Expected<'_>,
    operation: &IssuedLocalOperation,
    anchor: &mut IndependentAnchor,
) {
    let mut source_connection = connection(restart.file);
    let mut source_port = SqliteOperationCustody {
        connection: &mut source_connection,
        coordinate: expected.local.coordinate.clone(),
    };
    let sources: Vec<_> = publication::all(&mut source_port)
        .into_iter()
        .filter(|source| {
            if source.kind() != OperationPublicationSourceKind::A2aTaskObservation {
                return false;
            }
            canon::parse_strict(source.observation().canonical_record_jcs().as_slice())
                .unwrap()
                .get("task_binding_id")
                .and_then(serde_json::Value::as_str)
                == Some(expected.binding.task_binding_id().as_str())
        })
        .collect();
    assert_eq!(
        sources.len(),
        usize::try_from(expected.head.1.revision().get()).unwrap(),
        "every retained remote revision must be a publication source"
    );
    drop(source_port);
    drop(source_connection);

    let (producer, _) = InitialProducerFixture::new();
    let writer = publication_writer();
    let writer_ref = record_ref("publication/a2a-writer");
    let occurrence_schema =
        pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID).unwrap();
    let edge_schema = pale_contracts::record_schema_identity(GENEALOGY_EDGE_V1_SCHEMA_ID).unwrap();
    let frontier_schema =
        pale_contracts::record_schema_identity(RECONCILIATION_FRONTIER_V1_SCHEMA_ID).unwrap();
    let mut epoch = 100_u64;
    let mut committed_head = None;

    for replay in [false, true] {
        let signatures = Cell::new(0_usize);
        let mut established = Vec::new();
        for (source_index, source) in sources.iter().enumerate() {
            loop {
                epoch += 1;
                let mut operation_connection = connection(restart.file);
                let mut operation_port = SqliteOperationCustody {
                    connection: &mut operation_connection,
                    coordinate: expected.local.coordinate.clone(),
                };
                let selected_digest = A2aTaskObservationDigest::parse(
                    source.observation().coordinate().record_sha256().as_str(),
                )
                .unwrap();
                let history = recover_selected(
                    expected,
                    operation,
                    expected.binding.task_binding_id(),
                    None,
                    &mut operation_port,
                    anchor,
                    Limit::Exact,
                    A2aTaskObservationSelection::HistoryMember {
                        reference: source.observation().coordinate().record_ref(),
                        digest: &selected_digest,
                    },
                )
                .unwrap();
                assert_eq!(
                    usize::try_from(history.selected_observation().1.revision().get()).unwrap(),
                    source_index + 1
                );
                let checkpoint = admit_publication_checkpoint(source, expected);
                let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                    restart.location,
                    expected.local.coordinate.clone(),
                )
                .unwrap();
                let commit_observer = connection(restart.file);
                let data_version_before: i64 = commit_observer
                    .query_row("PRAGMA data_version", [], |row| row.get(0))
                    .unwrap();
                let mut resident =
                    open_publication_resident(&store, expected.local.coordinate.clone(), epoch);
                let mut event_port = &store;
                let mut admit_event = |_: &EventReferencePreimageV1| Ok(());
                let mut resolve_occurrence = |_: &ControlRecord| Ok(expected.local.signing.clone());
                let mut resolve_edge = |_: &ControlRecord| Ok(expected.local.signing.clone());
                let mut resolve_frontier = |_: &ControlRecord| Ok(expected.local.signing.clone());
                let mut verify_occurrence =
                    |domain: &[u8],
                     _: &InteractionOccurrencePreimageV1,
                     _: &InteractionOccurrenceDigest,
                     actual: &CanonicalSignatureV1| {
                        assert_eq!(domain, INTERACTION_OCCURRENCE_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        Ok(())
                    };
                let mut verify_edge =
                    |domain: &[u8],
                     _: &GenealogyEdgePreimageV1,
                     _: &GenealogyEdgeDigest,
                     actual: &CanonicalSignatureV1| {
                        assert_eq!(domain, GENEALOGY_EDGE_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        Ok(())
                    };
                let mut verify_frontier =
                    |domain: &[u8],
                     _: &ReconciliationFrontierPreimageV1,
                     _: &ReconciliationFrontierDigest,
                     actual: &CanonicalSignatureV1| {
                        assert_eq!(domain, RECONCILIATION_FRONTIER_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        Ok(())
                    };
                let mut readmission = LocalOperationPublicationReadmission {
                    admit_event_source: &mut admit_event,
                    resolve_occurrence_authority: &mut resolve_occurrence,
                    verify_occurrence_signature: &mut verify_occurrence,
                    resolve_edge_authority: &mut resolve_edge,
                    verify_edge_signature: &mut verify_edge,
                    resolve_frontier_authority: &mut resolve_frontier,
                    verify_frontier_signature: &mut verify_frontier,
                };
                let predecessor = if history.selected_observation().1.predecessor_ref().is_some() {
                    let predecessor_ref = history
                        .selected_observation()
                        .1
                        .predecessor_ref()
                        .as_ref()
                        .unwrap();
                    let predecessor_digest = history
                        .selected_observation()
                        .1
                        .predecessor_sha256()
                        .as_ref()
                        .unwrap();
                    let predecessor = recover_selected(
                        expected,
                        operation,
                        expected.binding.task_binding_id(),
                        None,
                        &mut operation_port,
                        anchor,
                        Limit::Exact,
                        A2aTaskObservationSelection::HistoryMember {
                            reference: predecessor_ref,
                            digest: predecessor_digest,
                        },
                    )
                    .unwrap();
                    Some(admit_retained_predecessor(
                        restart.file,
                        expected,
                        &predecessor,
                        operation,
                    ))
                } else {
                    None
                };
                let references = A2aTaskPublicationReferences::derive(
                    expected.local.coordinate,
                    history.selected_observation().0,
                    history.selected_observation().1,
                )
                .unwrap();
                let plan = admit_a2a_task_publication_plan(
                    expected.local.coordinate,
                    source,
                    &history,
                    operation,
                    (source.checkpoint().coordinate().record_ref(), &checkpoint),
                    &references,
                    predecessor,
                )
                .unwrap();
                let mut sign_occurrence =
                    |_: &[u8],
                     _: &InteractionOccurrencePreimageV1,
                     _: &InteractionOccurrenceDigest| {
                        signatures.set(signatures.get() + 1);
                        Ok(signature())
                    };
                let mut sign_edge =
                    |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
                        signatures.set(signatures.get() + 1);
                        Ok(signature())
                    };
                let mut sign_frontier =
                    |_: &[u8],
                     _: &ReconciliationFrontierPreimageV1,
                     _: &ReconciliationFrontierDigest| {
                        signatures.set(signatures.get() + 1);
                        Ok(signature())
                    };
                let mut issuance = LocalOperationPublicationIssuance {
                    sign_occurrence: &mut sign_occurrence,
                    sign_edge: &mut sign_edge,
                    sign_frontier: &mut sign_frontier,
                };
                let contexts = LocalOperationPublicationSigningContexts {
                    occurrence: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &occurrence_schema.schema_sha256,
                        signing_authority: expected.local.signing,
                    },
                    edge: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &edge_schema.schema_sha256,
                        signing_authority: expected.local.signing,
                    },
                    frontier: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &frontier_schema.schema_sha256,
                        signing_authority: expected.local.signing,
                    },
                };
                let race = |_: &EventPublicationRaceClosure<'_>| Ok(());
                let advanced = advance_a2a_task_publication(
                    &mut resident,
                    &mut event_port,
                    &plan,
                    &producer.allocation_ref,
                    &producer.allocation,
                    &writer_ref,
                    &writer,
                    EventPublicationState::Active,
                    &race,
                    contexts,
                    &mut issuance,
                    &mut readmission,
                    1 << 20,
                )
                .unwrap();
                let head = resident.journal_head().unwrap();
                drop(resident);
                let data_version_after: i64 = commit_observer
                    .query_row("PRAGMA data_version", [], |row| row.get(0))
                    .unwrap();
                match advanced {
                    LocalOperationPublicationAdvance::Established {
                        stage, evidence, ..
                    } => {
                        assert!(!replay, "cold replay cannot establish another stage");
                        assert_eq!(evidence, EventCustodyCommitEvidence::New);
                        established.push((source_index, stage));
                    }
                    LocalOperationPublicationAdvance::Complete => {
                        assert_eq!(data_version_after, data_version_before);
                        break;
                    }
                }
                committed_head = Some(head);
            }
        }

        if replay {
            assert!(established.is_empty());
            assert_eq!(signatures.get(), 0);
            let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                restart.location,
                expected.local.coordinate.clone(),
            )
            .unwrap();
            let resident =
                open_publication_resident(&store, expected.local.coordinate.clone(), epoch + 1);
            assert_eq!(resident.journal_head(), committed_head);
        } else {
            let expected_stages: Vec<_> = (0..sources.len())
                .flat_map(|index| {
                    let mut stages = vec![
                        (index, LocalOperationPublicationStage::Event),
                        (index, LocalOperationPublicationStage::Occurrence),
                    ];
                    if index > 0 {
                        stages.push((index, LocalOperationPublicationStage::GenealogyEdge));
                    }
                    stages.push((
                        index,
                        LocalOperationPublicationStage::ReconciliationFrontier,
                    ));
                    stages
                })
                .collect();
            assert_eq!(established, expected_stages);
            assert_eq!(signatures.get(), sources.len() * 2 + sources.len() - 1);
        }
    }

    assert_remote_publication_shape(restart.file, &sources);
}

fn admit_publication_checkpoint(
    source: &OperationPublicationSource,
    expected: &Expected<'_>,
) -> ControlStateCheckpointV1 {
    let schema =
        pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID).unwrap();
    let wire: ControlStateCheckpointWireV1 = serde_json::from_value(
        canon::parse_strict(source.checkpoint().canonical_record_jcs().as_slice()).unwrap(),
    )
    .unwrap();
    ControlStateCheckpointV1::admit_signed_wire(
        wire,
        &schema.schema_sha256,
        &checkpoint_authority(expected.local.checkpoint),
        verify_checkpoint,
    )
    .unwrap()
}

fn admit_retained_predecessor(
    file: &std::path::Path,
    expected: &Expected<'_>,
    predecessor: &RecoveredA2aTaskHistory,
    operation: &IssuedLocalOperation,
) -> A2aTaskPredecessorPublication {
    let event_schema =
        pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID).unwrap();
    let (event_ref, event) = retained_records(file, EVENT_REFERENCE_V1_SCHEMA)
        .into_iter()
        .map(|(reference, bytes)| {
            let wire: EventReferenceWireV1 = serde_json::from_slice(&bytes).unwrap();
            let event =
                EventReferenceV1::admit_contextual_wire(wire, &event_schema.schema_sha256, |_| {
                    Ok(())
                })
                .unwrap();
            (reference, event)
        })
        .find(|(_, event)| event.source_record_ref() == predecessor.selected_observation().0)
        .expect("predecessor Event is retained");
    let occurrence_schema =
        pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID).unwrap();
    let context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: &occurrence_schema.schema_sha256,
        signing_authority: expected.local.signing,
    };
    let (occurrence_ref, occurrence) = retained_records(file, INTERACTION_OCCURRENCE_V1_SCHEMA)
        .into_iter()
        .map(|(reference, bytes)| {
            let wire: InteractionOccurrenceWireV1 = serde_json::from_slice(&bytes).unwrap();
            let occurrence = InteractionOccurrenceV1::admit_contextual_wire(
                wire,
                &context,
                |_| Ok(()),
                |_, _, _, actual| {
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
            .unwrap();
            (reference, occurrence)
        })
        .find(|(_, occurrence)| occurrence.event_reference_ref() == &event_ref)
        .expect("predecessor occurrence is retained");
    admit_a2a_task_predecessor_publication(
        expected.local.coordinate,
        predecessor,
        operation,
        (&event_ref, &event),
        (&occurrence_ref, &occurrence),
        context,
        |_| Ok(()),
        |_, _, _, actual| {
            assert_eq!(actual, &signature());
            Ok(())
        },
    )
    .unwrap()
}

fn retained_records(file: &std::path::Path, schema: &str) -> Vec<(ResidentRecordRefV1, Vec<u8>)> {
    let connection = connection(file);
    let mut statement = connection
        .prepare(
            "SELECT record_ref, canonical_record_jcs FROM pale_deployment_control_record \
             WHERE schema_id=?1 ORDER BY inserted_transaction_sequence,record_ref",
        )
        .unwrap();
    statement
        .query_map([schema], |row| {
            let reference: String = row.get(0)?;
            let bytes: Vec<u8> = row.get(1)?;
            Ok((ResidentRecordRefV1::parse(reference).unwrap(), bytes))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn open_publication_resident<'a>(
    mut store: &'a DeploymentControlSqliteStore,
    coordinate: AuthorityStoreCoordinate,
    epoch: u64,
) -> ResidentApplication<'a> {
    let limits = AuthorityRecoveryReadLimits::new(1 << 20, 1 << 20, 8, 64).unwrap();
    let resolver = RecoveryResolver::default();
    let mut source = FixedInputSource::default();
    let opened = ResidentApplication::cold_open(
        store,
        store,
        &mut store,
        limits.maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            coordinate,
            limits,
            limits.maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &resolver,
            &mut source,
            ResidentReconstructionParameters::new(
                AdaptiveNodeSetPolicy::new(32, 5_000).unwrap(),
                8,
                8,
                WakeEpochV1::new(epoch),
            ),
        ),
    )
    .unwrap();
    match opened {
        ResidentColdOpenOutcome::Ready(resident) => *resident,
        ResidentColdOpenOutcome::Degraded(_) => panic!("valid publication custody must reopen"),
    }
}

fn assert_remote_publication_shape(file: &std::path::Path, sources: &[OperationPublicationSource]) {
    let connection = connection(file);
    let mut statement = connection
        .prepare(
            "SELECT schema_id, record_ref, canonical_record_jcs FROM pale_deployment_control_record \
             WHERE schema_id IN (?1,?2,?3,?4) ORDER BY inserted_transaction_sequence,record_ref",
        )
        .unwrap();
    let rows: Vec<(String, String, Vec<u8>)> = statement
        .query_map(
            rusqlite::params![
                EVENT_REFERENCE_V1_SCHEMA,
                INTERACTION_OCCURRENCE_V1_SCHEMA,
                GENEALOGY_EDGE_V1_SCHEMA,
                RECONCILIATION_FRONTIER_V1_SCHEMA,
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let count = |schema: &str| {
        rows.iter()
            .filter(|(actual, _, _)| actual == schema)
            .count()
    };
    assert_eq!(count(EVENT_REFERENCE_V1_SCHEMA), sources.len());
    assert_eq!(count(INTERACTION_OCCURRENCE_V1_SCHEMA), sources.len());
    assert_eq!(count(GENEALOGY_EDGE_V1_SCHEMA), sources.len() - 1);
    assert_eq!(count(RECONCILIATION_FRONTIER_V1_SCHEMA), sources.len());
    let values: Vec<_> = rows
        .iter()
        .map(|(schema, reference, bytes)| {
            (
                schema.as_str(),
                reference.as_str(),
                canon::parse_strict(bytes).unwrap(),
            )
        })
        .collect();
    for (schema, _, bytes) in &rows {
        let value = canon::parse_strict(bytes).unwrap();
        assert_no_remote_body(&value);
        let canonical = String::from_utf8(bytes.clone()).unwrap();
        for remote_meaning in [
            "\"remote_task_id\"",
            "\"remote_context_id\"",
            "\"working\"",
            "\"completed\"",
            "fixture-task-1",
            "GetTask",
            "TASK_STATE_WORKING",
            "TASK_STATE_COMPLETED",
            "fixture-state",
            "CancelTask",
        ] {
            assert!(
                !canonical.contains(remote_meaning),
                "publication promoted remote meaning {remote_meaning}"
            );
        }
        let object = value.as_object().unwrap();
        match schema.as_str() {
            EVENT_REFERENCE_V1_SCHEMA => {
                assert_eq!(
                    object.get("event_type").and_then(serde_json::Value::as_str),
                    Some("pale.a2a.task.observation.committed")
                );
                assert!(object
                    .get("payload_ref")
                    .is_none_or(serde_json::Value::is_null));
                assert!(object
                    .get("payload_sha256")
                    .is_none_or(serde_json::Value::is_null));
            }
            INTERACTION_OCCURRENCE_V1_SCHEMA => assert_eq!(
                object
                    .get("lifecycle")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|lifecycle| lifecycle.get("state"))
                    .and_then(serde_json::Value::as_str),
                Some("unknown")
            ),
            GENEALOGY_EDGE_V1_SCHEMA => assert_eq!(
                object
                    .get("proof")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|proof| proof.get("relation"))
                    .and_then(serde_json::Value::as_str),
                Some("successor")
            ),
            RECONCILIATION_FRONTIER_V1_SCHEMA => {
                let dimensions = object
                    .get("dimensions")
                    .and_then(serde_json::Value::as_array)
                    .unwrap();
                assert_eq!(dimensions.len(), 1);
                let dimension = dimensions[0].as_object().unwrap();
                assert_eq!(
                    dimension
                        .get("dimension")
                        .and_then(serde_json::Value::as_str),
                    Some("owner_publication")
                );
                assert_eq!(
                    dimension
                        .get("status")
                        .and_then(serde_json::Value::as_object)
                        .and_then(|status| status.get("state"))
                        .and_then(serde_json::Value::as_str),
                    Some("complete")
                );
            }
            _ => unreachable!(),
        }
    }

    let mut prior_occurrence_ref = None;
    for (index, source) in sources.iter().enumerate() {
        let revision = (index + 1).to_string();
        let observation_ref = source.observation().coordinate().record_ref().as_str();
        let observation =
            canon::parse_strict(source.observation().canonical_record_jcs().as_slice()).unwrap();
        let observation_digest = observation
            .get("canonical_a2a_task_observation_sha256")
            .and_then(serde_json::Value::as_str)
            .unwrap();
        let native_revision = format!("a2a-task-observation:{revision}:{observation_digest}");
        let event = only_publication_record(&values, EVENT_REFERENCE_V1_SCHEMA, |object| {
            object
                .get("source_record_ref")
                .and_then(serde_json::Value::as_str)
                == Some(observation_ref)
        });
        assert_eq!(
            event
                .2
                .get("source_version")
                .and_then(serde_json::Value::as_str),
            Some(revision.as_str())
        );
        let occurrence =
            only_publication_record(&values, INTERACTION_OCCURRENCE_V1_SCHEMA, |object| {
                object
                    .get("event_reference_ref")
                    .and_then(serde_json::Value::as_str)
                    == Some(event.1)
            });
        if let Some(prior) = prior_occurrence_ref {
            let edge = only_publication_record(&values, GENEALOGY_EDGE_V1_SCHEMA, |object| {
                object
                    .get("destination_occurrence_ref")
                    .and_then(serde_json::Value::as_str)
                    == Some(occurrence.1)
            });
            assert_eq!(
                edge.2
                    .get("source_occurrence_ref")
                    .and_then(serde_json::Value::as_str),
                Some(prior)
            );
        }
        let frontier =
            only_publication_record(&values, RECONCILIATION_FRONTIER_V1_SCHEMA, |object| {
                object
                    .get("source_occurrence_refs")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|references| {
                        references.len() == 1
                            && references[0]
                                .get("record_ref")
                                .and_then(serde_json::Value::as_str)
                                == Some(occurrence.1)
                    })
            });
        let frontier = frontier.2;
        let owner_revisions = frontier
            .get("owner_record_revisions")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert_eq!(owner_revisions.len(), 1);
        assert_eq!(
            owner_revisions[0]
                .get("record_ref")
                .and_then(serde_json::Value::as_str),
            Some(observation_ref)
        );
        assert_eq!(
            owner_revisions[0]
                .pointer("/native_revision/native_revision_token")
                .and_then(serde_json::Value::as_str),
            Some(native_revision.as_str())
        );
        assert_eq!(
            frontier
                .get("subject")
                .and_then(serde_json::Value::as_object)
                .and_then(|subject| subject.get("coordinate"))
                .and_then(serde_json::Value::as_object)
                .and_then(|coordinate| coordinate.get("record_ref"))
                .and_then(serde_json::Value::as_str),
            Some(observation_ref)
        );
        assert_eq!(
            frontier
                .get("subject")
                .and_then(serde_json::Value::as_object)
                .and_then(|subject| subject.get("coordinate"))
                .and_then(serde_json::Value::as_object)
                .and_then(|coordinate| coordinate.get("native_revision"))
                .and_then(serde_json::Value::as_object)
                .and_then(|native_revision| native_revision.get("native_revision_token"))
                .and_then(serde_json::Value::as_str),
            Some(native_revision.as_str())
        );
        prior_occurrence_ref = Some(occurrence.1);
    }
}

fn only_publication_record<'a, F>(
    values: &'a [(&str, &str, serde_json::Value)],
    schema: &str,
    matches: F,
) -> (
    &'a str,
    &'a str,
    &'a serde_json::Map<String, serde_json::Value>,
)
where
    F: Fn(&serde_json::Map<String, serde_json::Value>) -> bool,
{
    let mut matched = values.iter().filter_map(|(actual, reference, value)| {
        let object = value.as_object()?;
        (*actual == schema && matches(object)).then_some((*actual, *reference, object))
    });
    let selected = matched.next().expect("publication record");
    assert!(
        matched.next().is_none(),
        "publication record must be unique"
    );
    selected
}

fn assert_no_remote_body(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for key in [
                "source",
                "remote_state",
                "remote_task_id",
                "remote_context_id",
            ] {
                assert!(
                    !object.contains_key(key),
                    "publication copied A2A field {key}"
                );
            }
            for member in object.values() {
                assert_no_remote_body(member);
            }
        }
        serde_json::Value::Array(array) => {
            for member in array {
                assert_no_remote_body(member);
            }
        }
        _ => {}
    }
}

fn current_read_authorization_precedes_private_selection(
    request: &CommonRequestEnvelopeV1,
    generation: &AdmittedCompiledGeneration,
    operation: &IssuedLocalOperation,
    recovered: &RecoveredA2aTaskHistory,
    port: &mut SqliteOperationCustody<'_>,
) {
    let OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::Dispatched { evidence, .. },
    } = recovered.dispatch().1.state()
    else {
        panic!("the retained binding names the exact dispatch")
    };
    let coordinate = port.coordinate.clone();
    let selection = port
        .read_operation_record(
            &coordinate,
            &ControlRecordCoordinate::new(
                evidence.record_ref.clone(),
                ControlRecordDigest::parse(evidence.record_sha256.as_str()).unwrap(),
            ),
            65_536,
        )
        .unwrap()
        .unwrap();
    let attempt = |maximum_selection_bytes| OperationAttemptHistoryInput {
        operation,
        request,
        session: operation.historical_session_lease(),
        lease: operation.historical_operation_lease(),
        generation,
        dispatched: recovered.dispatch().1,
        selection: &selection,
        maximum_selection_bytes,
    };
    let request_id = A2aJsonRpcRequestIdV1::parse("get-readback-1").unwrap();
    let input = |maximum_selection_bytes, request_record_ref, response_record_ref| {
        A2aGetTaskReadbackInput {
            history: recovered,
            attempt: attempt(maximum_selection_bytes),
            request_id: &request_id,
            request_record_ref,
            response_record_ref,
            limits: A2aExchangeLimits {
                request_bytes: std::num::NonZeroUsize::new(4096).unwrap(),
                response_bytes: std::num::NonZeroUsize::new(4096).unwrap(),
                evidence_bytes: std::num::NonZeroUsize::new(8192).unwrap(),
                line_bytes: std::num::NonZeroUsize::new(1024).unwrap(),
                event_bytes: std::num::NonZeroUsize::new(2048).unwrap(),
                events: std::num::NonZeroUsize::new(1).unwrap(),
                queued_events: std::num::NonZeroUsize::new(1).unwrap(),
                connect_timeout: std::time::Duration::from_secs(1),
                read_timeout: std::time::Duration::from_secs(1),
                total_timeout: std::time::Duration::from_secs(2),
            },
            maximum_evidence_bytes: 8192,
        }
    };
    let calls = Cell::new(0);
    let authorized_at = std::cmp::max(
        *recovered.observation().1.observed_at(),
        *recovered
            .local_observation()
            .selected_head()
            .1
            .observed_at(),
    );
    let carrier = NoGetTaskCarrier::default();
    let clock_authorization_calls = Cell::new(0);
    let unavailable = start_a2a_get_task_readback(
        input(
            65_536,
            record_ref("a2a/no-clock-get-request"),
            record_ref("a2a/no-clock-get-response"),
        ),
        &carrier,
        &UnavailableReadbackClock,
        |_| {
            clock_authorization_calls.set(clock_authorization_calls.get() + 1);
            Ok(())
        },
    );
    assert!(matches!(
        unavailable,
        Err(A2aGetTaskReadbackError::Clock(_))
    ));
    assert_eq!(clock_authorization_calls.get(), 0);
    assert_eq!((carrier.prepares.get(), carrier.starts.get()), (0, 0));

    let backwards = UnixTimeV1::new(
        authorized_at
            .get()
            .checked_sub(1)
            .expect("retained A2A history has a positive observation time"),
    )
    .unwrap();
    let backwards = start_a2a_get_task_readback(
        input(
            65_536,
            record_ref("a2a/backwards-clock-get-request"),
            record_ref("a2a/backwards-clock-get-response"),
        ),
        &carrier,
        &ReadbackClock(backwards),
        |_| panic!("backward physical time must precede owner authorization"),
    );
    assert!(matches!(backwards, Err(A2aGetTaskReadbackError::Time)));
    assert_eq!((carrier.prepares.get(), carrier.starts.get()), (0, 0));

    let alias_calls = Cell::new(0);
    let aliased = start_a2a_get_task_readback(
        input(
            1,
            recovered.binding().0.clone(),
            record_ref("a2a/aliased-get-response"),
        ),
        &carrier,
        &ReadbackClock(authorized_at),
        |_| {
            alias_calls.set(alias_calls.get() + 1);
            Ok(())
        },
    );
    assert!(matches!(aliased, Err(A2aGetTaskReadbackError::Association)));
    assert_eq!(alias_calls.get(), 0);
    assert_eq!((carrier.prepares.get(), carrier.starts.get()), (0, 0));

    let denied = start_a2a_get_task_readback(
        input(
            1,
            record_ref("a2a/denied-get-request"),
            record_ref("a2a/denied-get-response"),
        ),
        &carrier,
        &ReadbackClock(authorized_at),
        |_| {
            calls.set(calls.get() + 1);
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "A2A GetTask fixture",
                invariant: "current caller read authorization",
            })
        },
    );
    assert!(matches!(
        denied,
        Err(A2aGetTaskReadbackError::Authorization(_))
    ));
    assert_eq!(calls.get(), 1);
    assert_eq!((carrier.prepares.get(), carrier.starts.get()), (0, 0));
    assert!(
        selection.canonical_record_jcs().as_slice().len() > 1,
        "the denied input must be undecodable"
    );

    let stale = UnixTimeV1::new(operation.handle().1.retain_until().get() + 1).unwrap();
    let stale = start_a2a_get_task_readback(
        input(
            65_536,
            record_ref("a2a/stale-get-request"),
            record_ref("a2a/stale-get-response"),
        ),
        &carrier,
        &ReadbackClock(stale),
        |_| panic!("retention failure must precede owner authorization"),
    );
    assert!(matches!(stale, Err(A2aGetTaskReadbackError::Time)));
    assert_eq!((carrier.prepares.get(), carrier.starts.get()), (0, 0));
}

#[derive(Default)]
struct NoGetTaskCarrier {
    prepares: Cell<usize>,
    starts: Cell<usize>,
}

impl A2aExchangePort for NoGetTaskCarrier {
    fn prepare(&self, _: A2aExchangeRequest) -> Result<A2aPreparedExchange, A2aExchangeError> {
        self.prepares.set(self.prepares.get() + 1);
        panic!("rejected GetTask inputs must not reach preparation")
    }

    fn start(&self, _: A2aPreparedExchange) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.set(self.starts.get() + 1);
        panic!("rejected GetTask inputs must not reach carrier start")
    }
}

struct ReadbackClock(UnixTimeV1);

impl WallClockSourcePort for ReadbackClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(self.0)
    }
}

struct UnavailableReadbackClock;

impl WallClockSourcePort for UnavailableReadbackClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Err(WallClockSourceError::from_source(std::io::Error::other(
            "fixture clock unavailable",
        )))
    }
}

struct ChangedAnchor<'a> {
    current: &'a mut IndependentAnchor,
    stale: AntiRollbackAnchor,
    reads: usize,
    switch_after: usize,
}

impl AntiRollbackAnchorPort for ChangedAnchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.reads += 1;
        if self.reads > self.switch_after {
            assert_eq!(key, &self.stale.key());
            Ok(Some(self.stale.clone()))
        } else {
            self.current.read(key)
        }
    }
    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("recovery never observes a write transition")
    }
    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("recovery never advances authority")
    }
}

fn corruptions(
    expected: &Expected<'_>,
    operation: &IssuedLocalOperation,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    candidate: &OperationCustodyCandidate,
) {
    let transaction = port.prepare_transaction(expected.local.coordinate).unwrap();
    let local_head = current_local(port, operation);
    let mut raw_records = vec![candidate.a2a_evidence()[0].clone()];
    if expected.binding.request_mode() == &A2aTaskRequestModeV1::SendStreamingMessage {
        let evidence = expected.binding.initial_task_evidence();
        let coordinate = ControlRecordCoordinate::new(
            evidence.record_ref.clone(),
            ControlRecordDigest::parse(evidence.record_sha256.as_str()).unwrap(),
        );
        let stream = port
            .read_a2a_evidence(expected.local.coordinate, &coordinate, 4096)
            .unwrap()
            .unwrap();
        assert_eq!(stream.sse(), Some(("event-1", "message")));
        raw_records.push(stream);
    }
    for raw in &raw_records {
        for update in [
            "evidence_bytes=zeroblob(length(evidence_bytes))",
            "sse_event_id='substituted',sse_event_type='message'",
        ] {
            port.connection.execute(&format!("UPDATE pale_deployment_operation_a2a_evidence SET {update} WHERE record_ref=?1"),
            [raw.coordinate().record_ref().as_str()]).unwrap();
            let writes = port.connection.total_changes();
            assert!(
                recover(
                    expected,
                    operation,
                    expected.binding.task_binding_id(),
                    None,
                    port,
                    anchor,
                    Limit::Exact
                )
                .is_err(),
                "{update}"
            );
            assert_eq!(port.connection.total_changes(), writes);
            let (id, kind) = raw
                .sse()
                .map_or((None, None), |(id, kind)| (Some(id), Some(kind)));
            port.connection.execute("UPDATE pale_deployment_operation_a2a_evidence SET evidence_bytes=?1,sse_event_id=?2,sse_event_type=?3 WHERE record_ref=?4",
            rusqlite::params![raw.bytes(), id, kind, raw.coordinate().record_ref().as_str()]).unwrap();
        }
    }
    let record = candidate
        .record(OperationCustodyRecordKind::A2aTaskObservation)
        .unwrap();
    let original = record.canonical_record_jcs().as_slice();
    let value: serde_json::Value = serde_json::from_slice(original).unwrap();
    // These are hostile physical wire rows, not newly signed contextual records.
    // The gap/cycle checks must terminate; canonical/signature/lineage rejection
    // is sufficient and never claimed as acceptance of an authenticated cycle.
    for (field, replacement) in [
        ("revision", serde_json::json!(99)),
        (
            "predecessor_ref",
            serde_json::json!(expected.head.0.as_str()),
        ),
        (
            "predecessor_ref",
            serde_json::json!("a2a/missing-predecessor"),
        ),
        ("task_binding_id", serde_json::json!("another-binding")),
    ] {
        let mut changed = value.clone();
        changed[field] = replacement;
        if changed["predecessor_ref"] == serde_json::json!(expected.head.0.as_str()) {
            changed["predecessor_sha256"] = serde_json::json!(expected
                .head
                .1
                .canonical_a2a_task_observation_sha256()
                .as_str());
        }
        let bytes = pale_core::canon::to_canonical_string(&changed).unwrap();
        port.connection.execute("UPDATE pale_deployment_control_record SET canonical_record_jcs=?1 WHERE record_ref=?2",
            rusqlite::params![bytes.as_bytes(), record.coordinate().record_ref().as_str()]).unwrap();
        let writes = port.connection.total_changes();
        assert!(
            recover(
                expected,
                operation,
                expected.binding.task_binding_id(),
                None,
                port,
                anchor,
                Limit::Exact
            )
            .is_err(),
            "{field}"
        );
        assert_eq!(port.connection.total_changes(), writes);
        port.connection.execute("UPDATE pale_deployment_control_record SET canonical_record_jcs=?1 WHERE record_ref=?2",
            rusqlite::params![original, record.coordinate().record_ref().as_str()]).unwrap();
    }
    for (role, selected, unlike) in [
        (
            "a2a_task_binding",
            &expected.inputs.binding_ref,
            &expected.inputs.route_ref,
        ),
        (
            "latest_a2a_task_observation",
            expected.head.0,
            &expected.inputs.observation_ref,
        ),
    ] {
        let digest = |reference: &ResidentRecordRefV1| -> String {
            port.connection
                .query_row(
                    "SELECT record_sha256 FROM pale_deployment_control_record WHERE record_ref=?1",
                    [reference.as_str()],
                    |row| row.get(0),
                )
                .unwrap()
        };
        let original_digest = digest(selected);
        let unlike_digest = digest(unlike);
        assert_eq!(port.connection.execute("UPDATE pale_deployment_operation_index SET record_ref=?1,record_sha256=?2 WHERE index_role=?3 AND record_ref=?4",
            rusqlite::params![unlike.as_str(), unlike_digest, role, selected.as_str()]).unwrap(), 1);
        let writes = port.connection.total_changes();
        assert!(
            recover(
                expected,
                operation,
                expected.binding.task_binding_id(),
                None,
                port,
                anchor,
                Limit::Exact
            )
            .is_err(),
            "{role}"
        );
        assert_eq!(port.connection.total_changes(), writes);
        assert_eq!(port.connection.execute("UPDATE pale_deployment_operation_index SET record_ref=?1,record_sha256=?2 WHERE index_role=?3 AND record_ref=?4",
            rusqlite::params![selected.as_str(), original_digest, role, unlike.as_str()]).unwrap(), 1);
    }
    assert_eq!(
        port.prepare_transaction(expected.local.coordinate).unwrap(),
        transaction
    );
    assert_eq!(current_local(port, operation), local_head);
    assert!(
        recover(
            expected,
            operation,
            expected.binding.task_binding_id(),
            None,
            port,
            anchor,
            Limit::Exact
        )
        .is_ok(),
        "exact restoration must recover again"
    );
}
