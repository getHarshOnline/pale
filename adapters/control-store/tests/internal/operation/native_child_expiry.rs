//! A cold root sweep repairs a descendant expiry in the shared checkpoint domain.
//! Stored expiry observations, authority, signatures and anchoring are fixtures.

use super::*;
use crate::resident_fixture::local_operation_start_fixture::RecoveryParents;
use pale_application::diagnostic::DiagnosticProjection;
use std::cell::RefCell;

#[derive(Clone, Copy)]
pub(super) enum Sweep {
    Provider(usize),
    Consult(usize),
}

impl Sweep {
    fn depth(self) -> usize {
        match self {
            Self::Provider(depth) | Self::Consult(depth) => depth,
        }
    }

    fn recover(
        self,
        process: &mut pale_application::resident::hook_response_restart_test_support::RestartFixture<'_>,
        bounded: (
            LocalOperationRecoveryContext<'_>,
            AccumulatorPreparationLimits,
        ),
        custody: &mut dyn OperationCustodyPort,
        sessions: &dyn SessionLeaseCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<OperationClassV1, String> {
        match self {
            Self::Provider(depth) => {
                process.cold_sweep_with_depth(bounded, depth, custody, sessions, anchor, admission)
            }
            Self::Consult(depth) => process.cold_consult_sweep_with_depth(
                bounded, depth, custody, sessions, anchor, admission,
            ),
        }
    }

    fn with_depth(self, depth: usize) -> Self {
        match self {
            Self::Provider(_) => Self::Provider(depth),
            Self::Consult(_) => Self::Consult(depth),
        }
    }
}

struct Child {
    operation: IssuedLocalOperation,
    request: CommonRequestEnvelopeV1,
    first_ref: ResidentRecordRefV1,
    first: OperationObservationV1,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint: ControlStateCheckpointV1,
}

fn start_child(
    fixture: &child_start::Fixture<'_>,
    depth: usize,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> Child {
    let custody = RefCell::new((port, anchor));
    child_start::with_delegate_start(
        fixture,
        |reference, lease| {
            let mut borrowed = custody.borrow_mut();
            let (port, anchor) = &mut *borrowed;
            let coordinate = port.coordinate.clone();
            retain_operation_lease_body(*port, &coordinate, reference, lease, 1 << 20).unwrap();
            child_start::prepare_parent(fixture, &[], port, anchor, || {}).unwrap()
        },
        &format!("expiry-depth-{depth}"),
        |prepared, request, _, _, _| {
            let mut borrowed = custody.borrow_mut();
            let (port, anchor) = &mut *borrowed;
            let prepared = prepared
                .unwrap_or_else(|error| panic!("child depth {depth}: {error:?}"))
                .prepare_checkpoint(
                    AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000).unwrap(),
                    *port,
                )
                .unwrap();
            let (prior_ref, prior) = prepared.predecessor().unwrap();
            let mut fields = fixture.checkpoint.clone();
            fields.control_checkpoint_id =
                ControlCheckpointIdV1::parse(format!("expiry-child-{depth}")).unwrap();
            fields.domain_sequence =
                ControlDomainSequenceV1::new(prior.domain_sequence().get() + 1).unwrap();
            fields.previous_control_checkpoint_ref = Some(prior_ref.clone());
            fields.previous_control_checkpoint_sha256 =
                Some(prior.canonical_control_checkpoint_sha256().clone());
            fields.committed_transaction_sequence = prepared.transaction_sequence();
            fields.state_root_sha256 = prepared.state_root().clone();
            fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
            fields.changed_leaf_keys = prepared.changed_keys().to_vec();
            fields.leaf_count = prepared.leaf_count();
            fields.issued_at = UnixTimeV1::new(66 + depth as u64).unwrap();
            let checkpoint_ref = record_ref(&format!("expiry/child-checkpoint-{depth}"));
            let (physical, pending) = prepared
                .issue_checkpoint(
                    &checkpoint_ref,
                    fields.clone(),
                    &fields.schema_sha256,
                    &checkpoint_authority(&fields),
                    |_, _, _| Ok(signature()),
                )
                .unwrap();
            let complete = physical
                .commit(*port, *anchor, 1 << 20)
                .unwrap()
                .advance_anchor(*anchor)
                .unwrap();
            let candidate = complete.candidate().clone();
            let operation = pending
                .finish(complete)
                .unwrap_or_else(|_| panic!("child start custody"));
            let first_record = candidate
                .record(OperationCustodyRecordKind::OperationObservation)
                .unwrap();
            let wire: OperationObservationWireV1 =
                serde_json::from_slice(first_record.canonical_record_jcs().as_slice()).unwrap();
            let first = issue_observation(
                fixture.parents.signing,
                operation.handle().0,
                operation.handle().1,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                *wire.revision(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                wire.observed_at().get(),
            );
            Child {
                operation,
                request: request.clone(),
                first_ref: first_record.coordinate().record_ref().clone(),
                first,
                checkpoint_ref,
                checkpoint: read_checkpoint(candidate.checkpoint().successor_record(), &fields),
            }
        },
    )
}

fn retain_expiry(
    source: (&IssuedLocalOperation, &CommonRequestEnvelopeV1),
    predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    signing: &SigningAuthorityBindingV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    anchor_now: bool,
) -> (
    ResidentRecordRefV1,
    OperationObservationV1,
    ResidentRecordRefV1,
    ControlStateCheckpointV1,
) {
    let (operation, request) = source;
    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let diagnostic = DiagnosticProjection {
        code: "operation_expired_before_dispatch".into(),
        stage: DiagnosticStageV1::OperationAdmission,
        disposition: DiagnosticDispositionV1::Blocked,
        summary: "operation expired before dispatch".into(),
        subject: Some((
            "operation".into(),
            operation.handle().1.operation_handle_id().as_str().into(),
        )),
        source: None,
        schema_reference: None,
        causes: Vec::new(),
        notes: Vec::new(),
        help: Vec::new(),
        facts: Vec::new(),
        retry: DiagnosticRetryV1::NotIndicated,
        effect: DiagnosticEffectCertaintyV1::NotStarted,
        request: Some((
            "operation_request".into(),
            request.canonical_request_sha256().as_str().into(),
        )),
        generation: None,
        receipt: None,
        omitted_causes: 0,
    }
    .admit(&identity)
    .unwrap();
    let suffix = operation.handle().1.operation_handle_id().as_str();
    let diagnostic_ref = record_ref(&format!("expiry/diagnostic-{suffix}"));
    let digest = ArtifactDigest::hash_bytes(diagnostic.canonical_bytes());
    let diagnostic_record = ControlRecord::new(
        ControlRecordCoordinate::new(
            diagnostic_ref.clone(),
            ControlRecordDigest::parse(digest.as_str()).unwrap(),
        ),
        identity.schema.clone(),
        identity.schema_sha256.clone(),
        ControlRecordBytes::new(diagnostic.canonical_bytes().to_vec()).unwrap(),
    );
    let reference = record_ref(&format!("expiry/observation-{suffix}"));
    let time = operation.handle().1.expires_at().get();
    let observation = issue_observation(
        signing,
        operation.handle().0,
        operation.handle().1,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        OperationObservationRevisionV1::new(predecessor.1.revision().get() + 1).unwrap(),
        Some(predecessor),
        OperationObservationStateV1::Expired {
            diagnostic: ImmutableRecordReferenceV1 {
                record_ref: diagnostic_ref,
                record_sha256: digest,
            },
        },
        time,
    );
    let coordinate = port.coordinate.clone();
    let append = pale_application::resident::hook_response_restart_test_support::append_with_diagnostic_fixture(
        OperationObservationTransition::new(
            operation.handle().0,
            operation.handle().1,
            predecessor.0,
            predecessor.1,
            &reference,
            &observation,
        ),
        &|_: &OperationObservationV1| Ok(coordinate.clone()),
        diagnostic_record,
    );
    let (physical, fields) = prepare_append(append, operation, prior, port, time);
    let pending = physical.commit(port, anchor).unwrap();
    let checkpoint = read_checkpoint(pending.candidate().checkpoint().successor_record(), &fields);
    let checkpoint_ref = pending
        .candidate()
        .checkpoint()
        .successor()
        .coordinate()
        .record_ref()
        .clone();
    if anchor_now {
        pending.advance_anchor(anchor).unwrap();
    } else {
        drop(pending);
    }
    (reference, observation, checkpoint_ref, checkpoint)
}

pub(super) fn run(
    fixture: child_start::Fixture<'_>,
    first: (&ResidentRecordRefV1, &OperationObservationV1),
    generation: super::super::local_recovery::GenerationInputs,
    sessions: &DeploymentControlSqliteStore,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    sweep: Sweep,
) {
    let depth = sweep.depth();
    if depth == 0 {
        refuse_consult_root_expiry(fixture, first, generation, sessions, port, anchor, sweep);
        return;
    }
    let mut children: Vec<Child> = Vec::new();
    for level in 1..=depth {
        let child = if let Some(parent) = children.last() {
            let parents = RecoveryParents {
                lease: parent.operation.historical_operation_lease(),
                ..*fixture.parents
            };
            start_child(
                &child_start::Fixture {
                    operation: &parent.operation,
                    request: &parent.request,
                    parents: &parents,
                    ..fixture
                },
                level,
                port,
                anchor,
            )
        } else {
            start_child(&fixture, level, port, anchor)
        };
        children.push(child);
    }
    let leaf = children.last().unwrap();
    let root_expiry = if fixture.operation.handle().1.operation_class() == &OperationClassV1::Invoke
    {
        Some(retain_expiry(
            (fixture.operation, fixture.request),
            first,
            (&leaf.checkpoint_ref, &leaf.checkpoint),
            fixture.parents.signing,
            port,
            anchor,
            true,
        ))
    } else {
        None
    };
    let prior = root_expiry
        .as_ref()
        .map_or((&leaf.checkpoint_ref, &leaf.checkpoint), |expiry| {
            (&expiry.2, &expiry.3)
        });
    let leaf_expiry = retain_expiry(
        (&leaf.operation, &leaf.request),
        (&leaf.first_ref, &leaf.first),
        prior,
        fixture.parents.signing,
        port,
        anchor,
        false,
    );
    let before = anchor.advance_count();
    let mut reopened = connection(fixture.file);
    reopened.pragma_update(None, "query_only", true).unwrap();
    let coordinate = port.coordinate.clone();
    let mut resumed = SqliteOperationCustody {
        connection: &mut reopened,
        coordinate: coordinate.clone(),
    };
    let mut process = service::hook_response::expiry::restart_process(
        sessions,
        &coordinate,
        fixture.parents.signing,
        generation,
    );
    let root_head = root_expiry
        .as_ref()
        .map_or(first, |expiry| (&expiry.0, &expiry.1));
    let mut known = vec![first, root_head, (&leaf_expiry.0, &leaf_expiry.1)];
    known.extend(
        children
            .iter()
            .map(|child| (&child.first_ref, &child.first)),
    );
    let input = history::Input {
        operation: fixture.operation,
        coordinate: &coordinate,
        selected_member: root_head,
        known: &known,
        checkpoint: fixture.checkpoint,
        signing: fixture.parents.signing,
    };
    let identities = children
        .iter()
        .map(|child| (&child.request, child.operation.handle().1))
        .collect::<Vec<_>>();
    let limits = AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap();
    for (maximum_depth, maximum_leaves) in [(depth - 1, 3 + 3 * depth), (depth, 2)] {
        let refused = history::with_descendant_admission(
            &input,
            fixture.request,
            fixture.parents,
            &identities,
            |mut context, admission| {
                context.membership_limits =
                    AccumulatorMembershipLimits::new(maximum_leaves, 10_000, 100_000).unwrap();
                sweep.with_depth(maximum_depth).recover(
                    &mut process,
                    (context, limits),
                    &mut resumed,
                    sessions,
                    anchor,
                    admission,
                )
            },
        );
        assert!(
            refused.is_err(),
            "bounded descendant repair must refuse incomplete proof"
        );
        assert_eq!(anchor.advance_count(), before);
        assert_eq!(resumed.connection.total_changes(), 0);
    }
    resumed
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    resumed
        .connection
        .execute_batch("SAVEPOINT missing_parent_link")
        .unwrap();
    assert_eq!(resumed.connection.execute(
        "DELETE FROM pale_deployment_operation_index WHERE index_role='parent_child_membership' AND record_ref=?1",
        [children[0].operation.handle().0.as_str()],
    ).unwrap(), 1);
    let fixture_writes = resumed.connection.total_changes();
    let refused = history::with_descendant_admission(
        &input,
        fixture.request,
        fixture.parents,
        &identities,
        |context, admission| {
            sweep.recover(
                &mut process,
                (context, limits),
                &mut resumed,
                sessions,
                anchor,
                admission,
            )
        },
    );
    assert!(
        refused.is_err(),
        "every ancestor requires its exact parent membership"
    );
    assert_eq!(anchor.advance_count(), before);
    assert_eq!(resumed.connection.total_changes(), fixture_writes);
    resumed
        .connection
        .execute_batch("ROLLBACK TO missing_parent_link; RELEASE missing_parent_link")
        .unwrap();
    resumed
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let recovered_root_class = history::with_descendant_admission(
        &input,
        fixture.request,
        fixture.parents,
        &identities,
        |context, admission| {
            sweep.recover(
                &mut process,
                (context, limits),
                &mut resumed,
                sessions,
                anchor,
                admission,
            )
        },
    )
    .unwrap();
    assert_eq!(
        &recovered_root_class,
        fixture.operation.handle().1.operation_class()
    );
    assert_eq!(anchor.advance_count(), before + 1);
    assert_eq!(resumed.connection.total_changes(), fixture_writes);
    if matches!(sweep, Sweep::Consult(_)) && recovered_root_class == OperationClassV1::Consult {
        let repeated = history::with_sweep_admission(
            &input,
            fixture.request,
            fixture.parents,
            |context, admission| {
                sweep.recover(
                    &mut process,
                    (context, limits),
                    &mut resumed,
                    sessions,
                    anchor,
                    admission,
                )
            },
        )
        .unwrap();
        assert_eq!(
            repeated,
            OperationClassV1::Consult,
            "unopened Consult source stays available"
        );
        assert_eq!(anchor.advance_count(), before + 1);
        assert_eq!(resumed.connection.total_changes(), fixture_writes);
    }
    assert_eq!(
        resumed
            .read_checkpoint_head(&coordinate, leaf_expiry.3.checkpoint_domain_id(), 1 << 20)
            .unwrap()
            .unwrap()
            .0
            .coordinate()
            .record_ref(),
        &leaf_expiry.2
    );
}

fn refuse_consult_root_expiry(
    fixture: child_start::Fixture<'_>,
    first: (&ResidentRecordRefV1, &OperationObservationV1),
    generation: super::super::local_recovery::GenerationInputs,
    sessions: &DeploymentControlSqliteStore,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    sweep: Sweep,
) {
    assert_eq!(
        fixture.operation.handle().1.operation_class(),
        &OperationClassV1::Consult
    );
    let coordinate = port.coordinate.clone();
    let (_, record) = port
        .read_checkpoint_head(
            &coordinate,
            &fixture.checkpoint.checkpoint_domain_id,
            1 << 20,
        )
        .unwrap()
        .unwrap();
    let prior = read_checkpoint(&record, fixture.checkpoint);
    let expired = retain_expiry(
        (fixture.operation, fixture.request),
        first,
        (record.coordinate().record_ref(), &prior),
        fixture.parents.signing,
        port,
        anchor,
        false,
    );
    let before = anchor.advance_count();
    let mut reopened = connection(fixture.file);
    reopened.pragma_update(None, "query_only", true).unwrap();
    let mut resumed = SqliteOperationCustody {
        connection: &mut reopened,
        coordinate: coordinate.clone(),
    };
    let mut process = service::hook_response::expiry::restart_process(
        sessions,
        &coordinate,
        fixture.parents.signing,
        generation,
    );
    let input = history::Input {
        operation: fixture.operation,
        coordinate: &coordinate,
        selected_member: (&expired.0, &expired.1),
        known: &[first, (&expired.0, &expired.1)],
        checkpoint: fixture.checkpoint,
        signing: fixture.parents.signing,
    };
    let result = history::with_sweep_admission(
        &input,
        fixture.request,
        fixture.parents,
        |context, admission| {
            sweep.recover(
                &mut process,
                (
                    context,
                    AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap(),
                ),
                &mut resumed,
                sessions,
                anchor,
                admission,
            )
        },
    );
    assert!(
        result.is_err(),
        "a Consult root cannot use Invoke/Delegate expiry repair"
    );
    assert_eq!(anchor.advance_count(), before);
    assert_eq!(resumed.connection.total_changes(), 0);
}
