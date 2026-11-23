//! Provider-backed Consult behavioral tests.

use super::*;
use crate::resident::process_host::consult_provider::*;

#[test]
fn provider_consult_bounded_turn_handles_pending_conclusive_and_unknown_without_replay() {
    super::native_invoke::run_consult_child_provider_invoke();
}

#[test]
fn provider_consult_same_process_resume_preserves_release_and_work_until_execution() {
    super::native_invoke::run_consult_child_provider_resume();
}

#[test]
fn provider_consult_recovered_reference_collision_returns_release_before_io() {
    super::native_invoke::run_consult_child_provider_recovered_reference_collision();
}

#[test]
fn provider_consult_cold_current_head_records_unknown_without_execution() {
    super::native_invoke::run_consult_child_provider_cold_current_head_loss();
}

#[test]
fn provider_consult_cold_cancellation_predecessor_records_unknown_without_execution() {
    super::native_invoke::run_consult_child_provider_cold_cancellation_predecessor_loss();
}

#[test]
fn provider_consult_cold_recovered_collisions_refuse_before_ports_or_reservation() {
    super::native_invoke::run_consult_child_provider_cold_recovered_reference_collision();
}

#[test]
fn provider_consult_cold_offered_continuation_retries_exact_owner_without_replay() {
    super::native_invoke::run_consult_child_provider_cold_offered_continuation();
}

#[test]
fn provider_consult_recording_continuation_is_lossless_without_replay() {
    super::native_invoke::run_consult_child_provider_recording_continuation();
}

#[test]
fn provider_consult_lost_recording_continuation_is_lossless_without_replay() {
    super::native_invoke::run_consult_child_provider_lost_recording_continuation();
}

#[test]
fn provider_consult_classification_continuation_is_lossless_without_replay() {
    super::native_invoke::run_consult_child_provider_classification_continuation();
}

#[test]
fn provider_consult_settlement_continuation_retains_classified_child() {
    super::native_invoke::run_consult_child_provider_settlement_continuation();
}

#[test]
fn provider_consult_retention_certainty_has_distinct_owned_phases() {
    let before = ResidentProcessConsultProviderChildRetentionFailure::Unopened(
        ResidentProcessConsultProviderUnopenedFailure::Admission(
            crate::resident::operation_dispatch::OperationAttemptError::Association,
        ),
    );
    let after = ResidentProcessConsultProviderChildRetentionFailure::Crossed(
        ResidentProcessConsultProviderCrossedFailure::Recovery(
            ResidentProcessOperationRestartError::State,
        ),
    );
    assert!(matches!(
        before,
        ResidentProcessConsultProviderChildRetentionFailure::Unopened(
            ResidentProcessConsultProviderUnopenedFailure::Admission(_)
        )
    ));
    assert!(matches!(
        after,
        ResidentProcessConsultProviderChildRetentionFailure::Crossed(
            ResidentProcessConsultProviderCrossedFailure::Recovery(_)
        )
    ));
}
