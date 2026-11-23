//! Local materialization settlement-law tests.

use pale_core::records::{DiagnosticDispositionV1, DiagnosticRetryV1};

use super::*;

#[test]
fn admission_diagnostics_distinguish_invalid_input_blocked_state_and_invariants() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let invalid = admission_failure(&api, LocalMaterializationError::InvalidDirectory("saved"));
    assert_eq!(
        invalid.diagnostic().disposition(),
        crate::DiagnosticDisposition::InvalidInput
    );
    assert_eq!(
        invalid.diagnostic().retry(),
        crate::DiagnosticRetry::AfterCorrection
    );
    assert_eq!(
        invalid.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::NotApplicable
    );

    assert_eq!(
        admission_policy(&LocalMaterializationError::EvidenceCapacity),
        (
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
        )
    );
    assert_eq!(
        admission_policy(&LocalMaterializationError::InternalInvariant("test law")),
        (
            DiagnosticDispositionV1::InternalFailure,
            DiagnosticRetryV1::NotIndicated,
        )
    );
}

#[test]
fn outcome_unknown_diagnostic_never_authorizes_an_automatic_retry() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let error = effect_unknown_failure(
        &api,
        LocalMaterializationError::InternalInvariant("injected restoration uncertainty"),
    );
    assert_eq!(
        error.diagnostic().disposition(),
        crate::DiagnosticDisposition::EffectOutcomeUnknown
    );
    assert_eq!(
        error.diagnostic().retry(),
        crate::DiagnosticRetry::ReconcileFirst
    );
    assert_eq!(
        error.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::OutcomeUnknown
    );
}

#[test]
fn restored_target_and_post_settlement_failures_report_confirmed_effect_results() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let target = failed_target_result(
        &api,
        "engine".to_owned(),
        LocalMaterializationError::InternalInvariant("injected restored target"),
        0,
        AuxiliaryEffectObservation::Confirmed,
    );
    assert_eq!(
        target
            .failure()
            .expect("failed target has a diagnostic")
            .effect(),
        crate::DiagnosticEffectCertainty::Confirmed
    );
    assert_eq!(
        fact_value(
            target.failure().expect("failed target has a diagnostic"),
            "coordination_effect",
        ),
        Some("confirmed".to_owned())
    );

    let result_failure = post_effect_failure(
        &api,
        LocalMaterializationError::InternalInvariant("injected run-result failure"),
    );
    assert_eq!(
        result_failure.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::Confirmed
    );
}

#[test]
fn evidence_preparation_distinguishes_unstarted_targets_from_uncertain_invalidation() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let untouched = evidence_preparation_failure(
        &api,
        PreparationFailure::new(
            LocalMaterializationError::InvalidDirectory("evidence"),
            AuxiliaryEffectObservation::Untouched,
        ),
    );
    assert_eq!(
        untouched.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::NotStarted
    );

    let capacity = evidence_preparation_failure(
        &api,
        PreparationFailure::new(
            LocalMaterializationError::EvidenceCapacity,
            AuxiliaryEffectObservation::Confirmed,
        ),
    );
    assert_eq!(
        capacity.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::Confirmed
    );

    let source = pale_adapter_fs::FsError::Io {
        operation: "synchronize run result removal",
        path: std::path::PathBuf::from("opaque-test-evidence"),
        source: std::io::Error::other("injected durability failure"),
    };
    let invalidation = evidence_preparation_failure(
        &api,
        PreparationFailure::new(
            LocalMaterializationError::Filesystem {
                operation: "invalidate prior run result",
                source: pale_adapter_fs::FsError::EffectUncertain {
                    operation: "remove prior run result",
                    path: std::path::PathBuf::from("opaque-test-evidence"),
                    source: Box::new(source),
                },
            },
            AuxiliaryEffectObservation::OutcomeUnknown,
        ),
    );
    assert_eq!(
        invalidation.diagnostic().disposition(),
        crate::DiagnosticDisposition::EffectOutcomeUnknown
    );
    assert_eq!(
        invalidation.diagnostic().retry(),
        crate::DiagnosticRetry::ReconcileFirst
    );
    assert_eq!(
        invalidation.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::OutcomeUnknown
    );
}

#[test]
fn plan_publication_diagnostics_scope_output_coordination_and_target_effects() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let untouched = plan_publication_failure(
        &api,
        PreparationFailure::new(
            LocalMaterializationError::InvalidDirectory("plan-output"),
            AuxiliaryEffectObservation::Untouched,
        ),
    );
    assert_eq!(
        untouched.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::NotStarted
    );
    assert_eq!(
        untouched.diagnostic().subject(),
        Some(("operation", "local_one_shot_plan_publication"))
    );
    assert_eq!(
        fact_value(untouched.diagnostic(), "output_coordination_effect"),
        Some("not_started".to_owned())
    );
    assert_eq!(
        fact_value(untouched.diagnostic(), "saved_plan_target_effect"),
        Some("not_started".to_owned())
    );

    let confirmed = plan_publication_failure(
        &api,
        PreparationFailure::new(
            LocalMaterializationError::InternalInvariant("injected projection failure"),
            AuxiliaryEffectObservation::Confirmed,
        ),
    );
    assert_eq!(
        confirmed.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::Confirmed
    );
    assert_eq!(
        fact_value(confirmed.diagnostic(), "output_coordination_effect"),
        Some("confirmed".to_owned())
    );
    assert_eq!(
        fact_value(confirmed.diagnostic(), "saved_plan_target_effect"),
        Some("not_started".to_owned())
    );

    let source = pale_adapter_fs::FsError::Io {
        operation: "synchronize saved plan",
        path: std::path::PathBuf::from("opaque-test-output"),
        source: std::io::Error::other("injected durability failure"),
    };
    let unknown = plan_publication_failure(
        &api,
        PreparationFailure::new(
            LocalMaterializationError::Filesystem {
                operation: "publish saved plan",
                source: pale_adapter_fs::FsError::EffectUncertain {
                    operation: "publish saved plan",
                    path: std::path::PathBuf::from("opaque-test-output"),
                    source: Box::new(source),
                },
            },
            AuxiliaryEffectObservation::OutcomeUnknown,
        ),
    );
    assert_eq!(
        unknown.diagnostic().disposition(),
        crate::DiagnosticDisposition::EffectOutcomeUnknown
    );
    assert_eq!(
        unknown.diagnostic().retry(),
        crate::DiagnosticRetry::ReconcileFirst
    );
    assert_eq!(
        unknown.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::OutcomeUnknown
    );
    assert_eq!(
        fact_value(unknown.diagnostic(), "effect_scope"),
        Some("saved_plan_target_and_output_coordination".to_owned())
    );
    assert_eq!(
        fact_value(unknown.diagnostic(), "saved_plan_target_effect"),
        Some("outcome_unknown".to_owned())
    );
}

fn fact_value(diagnostic: &crate::DiagnosticV1, key: &str) -> Option<String> {
    diagnostic
        .facts()
        .into_iter()
        .find(|fact| fact.key() == key)
        .map(|fact| fact.value().to_owned())
}
