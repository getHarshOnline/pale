//! Built-in computation across real `SQLite` transactions and native tree preparation.
//! The signer, current-authority callbacks and independent anchor remain explicit
//! test models. This does not admit deployment storage or a production key backend.

use super::local_recovery::{connection, with_host_generation, IndependentAnchor};
use crate::authority::DeploymentControlSqliteStore;
use crate::location::{ControlStorageRoot, ControlStoreScope, DeploymentStoreScope};
use crate::operation::SqliteOperationCustody;
use crate::resident_fixture::local_operation_start_fixture::StartAdmission;
use crate::resident_fixture::local_operation_start_fixture::{
    checkpoint_authority, checkpoint_preimage, issue_observation, native_request_body, record_ref,
    schema_digest, signature, store_coordinate, with_start_context, with_start_context_class,
};
use crate::resident_fixture::session_admission_fixture::{
    local_start_contract_set, LocalStartContext, ProvingAuthority,
};
use crate::store::{initialize_unadmitted_store, ControlStoreIdentity};
use pale_application::actions::{NativeActionInvocationError, NativeActionRegistry};
use pale_application::resident::*;
use pale_core::records::*;
use pale_ports::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Clock(AtomicUsize);

#[path = "native_invocation/journeys.rs"]
mod journeys;
#[path = "native_invocation/staged.rs"]
mod staged;
#[path = "native_invocation/terminal.rs"]
mod terminal;
use journeys::run_native_action_journeys;

/// Retained dispatch and fixture authority shared by outcome checks.
struct OutcomeFixture<'a> {
    journey: Journey,
    generation: &'a AdmittedCompiledGeneration,
    operation: &'a IssuedLocalOperation,
    request: &'a CommonRequestEnvelopeV1,
    parents: crate::resident_fixture::local_operation_start_fixture::RecoveryParents<'a>,
    coordinate: AuthorityStoreCoordinate,
    location: &'a crate::location::ControlStoreLocation,
    file: &'a std::path::Path,
    first_record: &'a ControlRecord,
    first: &'a OperationObservationV1,
    prepared_done: &'a CompletedOperationObservationCustody,
    preparation_history: (ResidentRecordRefV1, OperationObservationV1),
    dispatch_candidate: OperationCustodyCandidate,
    dispatch_checkpoint: ControlStateCheckpointV1,
    dispatch_history_owned: (ResidentRecordRefV1, OperationObservationV1),
    observation: OperationObservationAdmissionContextV1<'a>,
    registry: &'a NativeActionRegistry,
    contracts: &'a ContractSetV1,
    clock: &'a Clock,
    anchor: IndependentAnchor,
}

#[path = "native_cancellation.rs"]
mod cancellation;
#[path = "native_control_cancellation.rs"]
mod control_cancellation;
#[path = "native_refusal_cancellation.rs"]
mod refusal_cancellation;
#[path = "native_refusal_expiry.rs"]
mod refusal_expiry;

#[path = "native_a2a_custody.rs"]
mod a2a_custody;
#[path = "native_a2a_send.rs"]
mod a2a_send;
#[path = "native_child_expiry.rs"]
mod child_expiry;
#[path = "native_child_start.rs"]
pub(super) mod child_start;
#[path = "native_consult_expiry.rs"]
mod consult_expiry;
#[path = "native_history.rs"]
mod history;
#[path = "provider_service.rs"]
mod provider_service;
#[path = "native_service.rs"]
mod service;

#[derive(Clone, Copy, Debug)]
enum Journey {
    Service(service::Case),
    HookResponse(service::HookResponseCase),
    ChildExpiry(usize),
    ConsultChildExpiry(usize),
    ConsultExpiry,
    ConsultExpiryWithIntent,
    ConsultSweepChildExpiry(usize),
    ConsultSweepInvokeExpiry(usize),
    RecoveredService,
    CrashRecovery,
    StagedRecoverySuccess,
    StagedRecoveryFailure,
    StagedRecoveryAfterCancellation,
    StagedOutcomeSubstitution,
    StagedFailureDiagnosticSubstitution,
    StagedFailureRequestSubstitution,
    StagedFailureRequestKindSubstitution,
    StagedFailureRequestMissing,
    StagedOutcomeMissingIndex,
    StagedOutcomeMissingEvidence,
    StagedOutcomeMalformedEvidence,
    StagedOutcomeWrongSchema,
    A2aCustody,
    A2aStreamingRecovery,
    Success,
    SuccessAfterExpiry,
    FailureAfterUnknown,
    BoundedReadRefusal,
    RefusalCancellation,
    RefusalExpiry,
    ChildStartWins,
    ChildServiceSuccess,
    ParentCancellationWins,
    ChildStartTransactionChanges,
    UnusedReadAuthority,
    EvidenceOnly,
    ReadAndEvidence,
    DispatchReplay,
    HandlerFailure,
    CancellationWins,
    DispatchWins,
    ControlCancellationWins,
    ControlDispatchWins,
    ControlFailureAfterCancellation,
    ReadAuthorityCancellationWins,
    ReadAuthorityDispatchWins,
}

enum TerminalEvidence<'a> {
    Succeeded(Box<PreparedNativeOperationCompletion<'a>>),
    Failed(Box<PreparedNativeOperationFailure<'a>>),
}

impl TerminalEvidence<'_> {
    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        match self {
            Self::Succeeded(value) => value.observation(),
            Self::Failed(value) => value.observation(),
        }
    }

    fn prepare_append(
        &self,
        resolve: &dyn OperationObservationAuthorityStoreResolver,
    ) -> PreparedOperationObservationAppend<'_> {
        match self {
            Self::Succeeded(value) => value.prepare_append(resolve).unwrap(),
            Self::Failed(value) => value.prepare_append(resolve).unwrap(),
        }
    }
}

impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let step = self.0.fetch_add(1, Ordering::SeqCst);
        assert!(step < 3);
        Ok(UnixTimeV1::new(65 + step as u64).unwrap())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RetainedCustodyShape {
    records: i64,
    indexes: i64,
    observations: i64,
    native_outcomes: i64,
    native_outcome_indexes: i64,
    results: i64,
    diagnostics: i64,
}

fn retained_custody_shape(connection: &rusqlite::Connection) -> RetainedCustodyShape {
    let count = |table: &str| {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    };
    let count_schema = |schema: &str| {
        connection
            .query_row(
                "SELECT COUNT(*) FROM pale_deployment_control_record WHERE schema_id=?1",
                [schema],
                |row| row.get(0),
            )
            .unwrap()
    };
    RetainedCustodyShape {
        records: count("pale_deployment_control_record"),
        indexes: count("pale_deployment_operation_index"),
        observations: count_schema(OPERATION_OBSERVATION_V1_SCHEMA),
        native_outcomes: count_schema(PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA),
        native_outcome_indexes: connection
            .query_row(
                "SELECT COUNT(*) FROM pale_deployment_operation_index WHERE index_role='native_outcome'",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        results: count_schema("pale/action-result.v1"),
        diagnostics: count_schema(DIAGNOSTIC_V1_SCHEMA),
    }
}

fn read_checkpoint(
    record: &ControlRecord,
    fields: &ControlStateCheckpointPreimageV1,
) -> ControlStateCheckpointV1 {
    ControlStateCheckpointV1::admit_signed_wire(
        serde_json::from_slice(record.canonical_record_jcs().as_slice()).unwrap(),
        &fields.schema_sha256,
        &checkpoint_authority(fields),
        |domain, _, _, actual| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
    )
    .unwrap()
}

fn prepare_append(
    append: PreparedOperationObservationAppend<'_>,
    operation: &IssuedLocalOperation,
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    custody: &mut dyn OperationCustodyPort,
    time: u64,
) -> (
    PreparedOperationObservationCustody,
    ControlStateCheckpointPreimageV1,
) {
    let reference = record_ref(&format!(
        "operation/checkpoint-{}",
        prior.1.domain_sequence().get() + 1
    ));
    prepare_append_with_ref(append, operation, prior, custody, time, &reference)
}

pub(super) fn prepare_append_with_ref(
    append: PreparedOperationObservationAppend<'_>,
    operation: &IssuedLocalOperation,
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    custody: &mut dyn OperationCustodyPort,
    time: u64,
    reference: &ResidentRecordRefV1,
) -> (
    PreparedOperationObservationCustody,
    ControlStateCheckpointPreimageV1,
) {
    let prepared = append
        .prepare_checkpoint(
            operation.checkpoint_selection(),
            prior.0,
            prior.1,
            AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap(),
            custody,
        )
        .unwrap();
    let sequence = prior.1.domain_sequence().get() + 1;
    let mut fields = checkpoint_preimage(
        sequence,
        Some((
            prior.0.clone(),
            prior.1.canonical_control_checkpoint_sha256().clone(),
        )),
    );
    fields.checkpoint_domain_id = prior.1.checkpoint_domain_id().clone();
    fields.checkpoint_profile_ref = prior.1.checkpoint_profile_ref().clone();
    fields.checkpoint_profile_sha256 = prior.1.checkpoint_profile_sha256().clone();
    fields.committed_transaction_sequence = prepared.transaction_sequence();
    fields.state_root_sha256 = prepared.state_root().clone();
    fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    fields.changed_leaf_keys = vec![prepared.observation_key().clone()];
    fields.leaf_count = *prior.1.leaf_count();
    fields.issued_at = UnixTimeV1::new(time).unwrap();
    let (physical, _) = prepared
        .issue_checkpoint(
            reference,
            fields.clone(),
            &fields.schema_sha256,
            &checkpoint_authority(&fields),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    (physical, fields)
}

#[test]
fn native_action_commits_selection_dispatch_and_terminal_evidence_over_reopened_trees() {
    run_native_action_journeys(&[
        Journey::Service(service::Case::Success),
        Journey::Service(service::Case::HandlerFailure),
        Journey::Service(service::Case::ChangedSession),
        Journey::Service(service::Case::AliasedCheckpoint),
        Journey::Service(service::Case::PreparationCheckpointFailure),
        Journey::Service(service::Case::DispatchSignerFailure),
        Journey::Service(service::Case::ReleaseClockMissing),
        Journey::Service(service::Case::CompletionClockMissing),
        Journey::Service(service::Case::CompletionClockBackwards),
        Journey::Service(service::Case::ResultSignerFailure),
        Journey::Service(service::Case::TerminalCheckpointFailure),
        Journey::Service(service::Case::TerminalCommitFailure),
        Journey::Service(service::Case::DispatchLostConfirmation),
        Journey::Service(service::Case::DispatchUnknownReadBack),
        Journey::Service(service::Case::OutcomeUnknownReadBack),
        Journey::Service(service::Case::OutcomeAnchorFailure),
        Journey::Service(service::Case::TerminalAnchorFailure),
        Journey::Service(service::Case::PreparationAnchorFailure),
        Journey::Service(service::Case::DispatchAnchorFailure),
        Journey::Service(service::Case::EvidenceObligations),
        Journey::Service(service::Case::UnusedReadAuthority),
        Journey::RecoveredService,
        Journey::CrashRecovery,
        Journey::StagedRecoverySuccess,
        Journey::StagedRecoveryFailure,
        Journey::StagedRecoveryAfterCancellation,
        Journey::StagedOutcomeSubstitution,
        Journey::StagedFailureDiagnosticSubstitution,
        Journey::StagedFailureRequestSubstitution,
        Journey::StagedFailureRequestKindSubstitution,
        Journey::StagedFailureRequestMissing,
        Journey::A2aCustody,
        Journey::A2aStreamingRecovery,
        Journey::Success,
        Journey::SuccessAfterExpiry,
        Journey::FailureAfterUnknown,
        Journey::BoundedReadRefusal,
        Journey::RefusalCancellation,
        Journey::RefusalExpiry,
        Journey::ChildStartWins,
        Journey::ChildServiceSuccess,
        Journey::ParentCancellationWins,
        Journey::ChildStartTransactionChanges,
        Journey::UnusedReadAuthority,
        Journey::EvidenceOnly,
        Journey::ReadAndEvidence,
        Journey::DispatchReplay,
        Journey::HandlerFailure,
        Journey::CancellationWins,
        Journey::DispatchWins,
        Journey::ControlCancellationWins,
        Journey::ControlDispatchWins,
        Journey::ControlFailureAfterCancellation,
        Journey::ReadAuthorityCancellationWins,
        Journey::ReadAuthorityDispatchWins,
    ]);
}

#[test]
fn staged_native_outcome_missing_index_or_evidence_fails_closed_without_replay() {
    run_native_action_journeys(&[
        Journey::StagedOutcomeMissingIndex,
        Journey::StagedOutcomeMissingEvidence,
    ]);
}

#[test]
fn staged_native_outcome_malformed_bytes_or_wrong_schema_fail_closed_without_replay() {
    run_native_action_journeys(&[
        Journey::StagedOutcomeMalformedEvidence,
        Journey::StagedOutcomeWrongSchema,
    ]);
}

#[test]
fn native_failure_evidence_requires_request_correlation_after_restart() {
    run_native_action_journeys(&[
        Journey::HandlerFailure,
        Journey::StagedFailureRequestSubstitution,
        Journey::StagedFailureRequestKindSubstitution,
        Journey::StagedFailureRequestMissing,
    ]);
}

#[test]
fn native_response_sqlite_continues_exact_rollback_and_anchor_phases() {
    run_native_action_journeys(&[
        Journey::HookResponse(service::HookResponseCase::Rollback),
        Journey::HookResponse(service::HookResponseCase::Anchor),
    ]);
}

#[test]
fn native_response_sqlite_recovered_dispatch_cannot_release_again() {
    run_native_action_journeys(&[Journey::HookResponse(
        service::HookResponseCase::UncertainDispatch,
    )]);
}

#[test]
fn native_response_cold_sqlite_reconstructs_preparation_and_dispatch_after_expiry() {
    run_native_action_journeys(&[
        Journey::HookResponse(service::HookResponseCase::ColdPrepared),
        Journey::HookResponse(service::HookResponseCase::ColdDispatched),
    ]);
}

#[test]
fn native_response_cold_sqlite_rejects_saved_request_and_handle_substitution() {
    run_native_action_journeys(&[
        Journey::HookResponse(service::HookResponseCase::ColdWrongRequest),
        Journey::HookResponse(service::HookResponseCase::ColdWrongHandle),
    ]);
}

#[test]
fn native_response_cold_sqlite_classifier_expires_through_interrupted_custody() {
    run_native_action_journeys(&[
        Journey::HookResponse(service::HookResponseCase::ColdExpiryRollback),
        Journey::HookResponse(service::HookResponseCase::ColdExpiryAnchor),
    ]);
}

#[test]
fn native_response_cold_sweep_discovers_and_repairs_committed_expiry() {
    run_native_action_journeys(&[Journey::HookResponse(
        service::HookResponseCase::ColdExpiryHost,
    )]);
}

#[test]
fn native_cold_sweep_repairs_child_and_deeper_descendant_expiry() {
    run_native_action_journeys(&[Journey::ChildExpiry(1), Journey::ChildExpiry(3)]);
}

#[test]
fn native_cold_sweep_repairs_descendant_expiry_under_consult_root() {
    run_native_action_journeys(&[
        Journey::ConsultChildExpiry(1),
        Journey::ConsultChildExpiry(3),
    ]);
}

#[test]
fn native_cold_sweep_refuses_to_expire_consult_root() {
    run_native_action_journeys(&[Journey::ConsultChildExpiry(0)]);
}

#[test]
fn native_consult_sweep_repairs_descendants_before_any_provider_sweep() {
    run_native_action_journeys(&[
        Journey::ConsultSweepChildExpiry(1),
        Journey::ConsultSweepChildExpiry(3),
        Journey::ConsultSweepInvokeExpiry(1),
    ]);
}

#[test]
fn native_consult_sweep_refuses_root_consult_expiry() {
    run_native_action_journeys(&[Journey::ConsultSweepChildExpiry(0)]);
}
