//! Terminal diagnostic projection laws over API-admitted failures.
//!
//! These tests prove that human and machine renderers preserve one canonical
//! diagnostic owner instead of reconstructing application failure policy.

use anyhow::anyhow;
use pale_sdk::{
    ContractDocument, DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticProjection,
    DiagnosticProjectionSource, DiagnosticRetry, DiagnosticStage, NativeActionRequest, PaleApi,
};

use super::*;

use crate::authoring_failure_fixture as authoring_fixture;

fn authoring_error(documents: Vec<(&str, String)>) -> ApiError {
    PaleApi::new()
        .unwrap()
        .compile_portable_workspace(pale_sdk::PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_owned(),
            documents
                .into_iter()
                .map(|(label, content)| {
                    pale_sdk::PortableWorkspaceDocument::new(label.to_owned(), content.into_bytes())
                })
                .collect(),
        ))
        .expect_err("the real compiler rejects the supplied authoring fixture")
}

#[test]
fn terminal_retains_failed_authoring_evidence_without_changing_its_exit() {
    for contract_rejection in [false, true] {
        let mut documents = authoring_fixture::documents();
        if contract_rejection {
            documents[0].1 = documents[0]
                .1
                .replace("schema_version = 1", "schema_version = 2");
        }
        let error = authoring_error(documents);
        let expected = DiagnosticPresentation::from_api_error(&error)
            .to_json()
            .unwrap();
        let corrections = serde_json::to_string_pretty(error.authoring_corrections()).unwrap();
        let location = serde_json::to_string_pretty(error.authoring_location().unwrap()).unwrap();
        let diagnostic = Diagnostic::from_api_error(&error);
        let wrapped =
            Diagnostic::from_host_error(&anyhow::Error::new(error).context("host wrapper"));
        assert_eq!(diagnostic.exit_code(), std::process::ExitCode::from(4));
        assert_eq!(wrapped.exit_code(), diagnostic.exit_code());
        assert_eq!(diagnostic.render_json().unwrap(), expected);
        assert_eq!(wrapped.render_json().unwrap(), expected);
        let human = diagnostic.render_human().unwrap();
        assert!(human.contains("Source location:"));
        for line in location.lines() {
            assert!(human.contains(&format!("\n    {line}")));
        }
        assert_eq!(
            human.contains("Suggested edits (not applied):"),
            !contract_rejection
        );
        if !contract_rejection {
            for line in corrections.lines() {
                assert!(human.contains(&format!("\n    {line}")));
            }
            assert!(human.contains("Match the current source"));
        }
        assert!(!human.contains("host wrapper"));
    }
}

#[test]
fn unknown_field_rejection_keeps_the_compiler_parent_location() {
    let mut documents = authoring_fixture::documents();
    documents[0].1 = documents[0]
        .1
        .replace("[workspace]", "[workspace]\n\"unselected\\nfield\" = 1");
    let error = authoring_error(documents);
    let location = serde_json::to_value(error.authoring_location().unwrap()).unwrap();
    assert_eq!(
        location["path"],
        serde_json::json!([{"kind":"field", "value":"workspace"}])
    );
    let diagnostic = Diagnostic::from_api_error(&error);
    let envelope: serde_json::Value =
        serde_json::from_str(&diagnostic.render_json().unwrap()).unwrap();
    assert_eq!(envelope["authoringLocation"], location);
    let human = diagnostic.render_human().unwrap();
    for line in serde_json::to_string_pretty(error.authoring_location().unwrap())
        .unwrap()
        .lines()
    {
        assert!(human.contains(&format!("\n    {line}")));
    }
}

fn effect_unknown(message: impl Into<String>, source: Error) -> Error {
    ClassifiedHostFailure {
        class: FailureClass::EffectUnknown,
        message: message.into(),
        source,
    }
    .into()
}

fn diagnostic_value(diagnostic: &Diagnostic) -> serde_json::Value {
    let envelope: serde_json::Value =
        serde_json::from_str(&diagnostic.render_json().unwrap()).unwrap();
    envelope["diagnostic"].clone()
}

fn admitted_value(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap()
}

fn admitted_diagnostic(projection: DiagnosticProjection, class: FailureClass) -> Diagnostic {
    let admitted = PaleApi::new()
        .unwrap()
        .project_diagnostic(projection)
        .unwrap();
    Diagnostic {
        document: DiagnosticDocument::Admitted {
            presentation: Box::new(DiagnosticPresentation::from_diagnostic(&admitted)),
            reconciliation: None,
        },
        class,
    }
}

#[test]
fn api_diagnostics_remain_the_only_semantic_failure_authority() {
    let source = PaleApi::new()
        .unwrap()
        .invoke_native_action(NativeActionRequest::new(
            "pale.compute.unknown".to_string(),
            b"null".to_vec(),
        ))
        .expect_err("unknown action is rejected by the public facade");
    let expected = admitted_value(source.diagnostic().canonical_bytes());

    let direct = Diagnostic::from_api_error(&source);
    assert_eq!(diagnostic_value(&direct), expected);
    assert_eq!(direct.exit_code(), std::process::ExitCode::from(4));

    let wrapped = anyhow::Error::new(source).context("host wrapper");
    let wrapped = Diagnostic::from_host_error(&wrapped);
    assert_eq!(diagnostic_value(&wrapped), expected);
    assert_eq!(wrapped.exit_code(), std::process::ExitCode::from(4));
}

#[test]
fn a_host_wrapper_cannot_widen_an_api_failure_to_effect_unknown() {
    let source = PaleApi::new()
        .unwrap()
        .invoke_native_action(NativeActionRequest::new(
            "pale.compute.unknown".to_string(),
            b"null".to_vec(),
        ))
        .expect_err("unknown action is rejected by the public facade");
    let expected = admitted_value(source.diagnostic().canonical_bytes());
    let wrapped = effect_unknown(
        "outer wrapper must not replace API policy",
        anyhow::Error::new(source),
    );
    let diagnostic = Diagnostic::from_host_error(&wrapped);
    let value = diagnostic_value(&diagnostic);

    assert_eq!(diagnostic_value(&diagnostic), expected);
    assert_eq!(value["disposition"], "invalid_input");
    assert_eq!(value["retry"], "after_correction");
    assert_eq!(value["effect"], "not_applicable");
    assert!(!diagnostic.render_human().unwrap().contains("Do not repeat"));
}

#[test]
fn api_human_rendering_uses_the_exact_admitted_schema_context() {
    let source = PaleApi::new()
        .unwrap()
        .validate_contract_document(ContractDocument::new(
            "diagnostic.v1".to_string(),
            b"{}".to_vec(),
        ))
        .expect_err("an empty diagnostic violates its generated contract");
    let expected: serde_json::Value =
        serde_json::from_slice(source.diagnostic().canonical_bytes()).unwrap();
    let diagnostic = Diagnostic::from_api_error(&source);
    let human = diagnostic.render_human().unwrap();

    for value in [
        expected["schema_coordinate"]["registry_id"]
            .as_str()
            .unwrap(),
        expected["schema_coordinate"]["schema_sha256"]
            .as_str()
            .unwrap(),
        expected["schema_coordinate"]["schema_path"]
            .as_str()
            .unwrap(),
    ] {
        assert!(human.contains(value), "human output omitted {value}");
    }
    for fact in expected["facts"].as_array().unwrap() {
        assert!(human.contains(fact["key"].as_str().unwrap()));
        assert!(human.contains(fact["value"].as_str().unwrap()));
    }
}

#[test]
fn unclassified_host_errors_remain_host_operational_failures() {
    let error = anyhow!("read failed").context("capture host input");
    let diagnostic = Diagnostic::from_host_error(&error);
    let value = diagnostic_value(&diagnostic);

    assert_eq!(diagnostic.exit_code(), std::process::ExitCode::from(1));
    assert_eq!(value["code"], "host_operation_failed");
    assert_eq!(value["stage"], "host_observation");
    assert_eq!(value["disposition"], "operational_failure");
    assert_eq!(value["summary"], "capture host input");
    assert_eq!(value["causes"], serde_json::json!(["read failed"]));
}

#[test]
fn classified_host_input_is_not_general_application_policy() {
    let error = invalid_input(
        "invalid confined action input",
        anyhow!("path escaped root"),
    );
    let diagnostic = Diagnostic::from_host_error(&error);
    let value = diagnostic_value(&diagnostic);

    assert_eq!(diagnostic.exit_code(), std::process::ExitCode::from(4));
    assert_eq!(value["code"], "host_input_rejected");
    assert_eq!(value["stage"], "input_admission");
    assert_eq!(value["disposition"], "invalid_input");
    assert_eq!(value["retry"], "after_correction");
    assert_eq!(value["effect"], "not_applicable");
}

#[test]
fn closed_cli_dispatch_contradictions_are_internal_failures() {
    let error = internal_failure(
        "the dispatcher returned an incompatible outcome",
        anyhow!("expected one closed outcome variant"),
    );
    let diagnostic = Diagnostic::from_host_error(&error);
    let value = diagnostic_value(&diagnostic);

    assert_eq!(diagnostic.exit_code(), std::process::ExitCode::from(6));
    assert_eq!(value["code"], "host_internal_failure");
    assert_eq!(value["stage"], "internal");
    assert_eq!(value["disposition"], "internal_failure");
    assert_eq!(value["retry"], "not_indicated");
    assert_eq!(value["effect"], "not_applicable");
}

#[test]
fn explicit_effect_uncertainty_requires_reconciliation() {
    let error = effect_unknown(
        "effect evidence is uncertain",
        anyhow!("storage observation"),
    );
    let diagnostic = Diagnostic::from_host_error(&error);
    let value = diagnostic_value(&diagnostic);

    assert_eq!(diagnostic.exit_code(), std::process::ExitCode::from(7));
    assert_eq!(value["code"], "effect_outcome_unknown");
    assert_eq!(value["disposition"], "effect_outcome_unknown");
    assert_eq!(value["retry"], "reconcile_first");
    assert_eq!(value["effect"], "outcome_unknown");
    assert!(diagnostic.render_human().unwrap().contains("Do not repeat"));
}

#[test]
fn uncertain_materialization_keeps_read_back_in_the_same_terminal_document() {
    let admitted = PaleApi::new()
        .unwrap()
        .project_diagnostic(
            DiagnosticProjection::new(
                "materialization_outcome_unknown".to_owned(),
                DiagnosticStage::Materialization,
                DiagnosticDisposition::EffectOutcomeUnknown,
                "Materialization outcome is unknown".to_owned(),
                DiagnosticRetry::ReconcileFirst,
                DiagnosticEffectCertainty::OutcomeUnknown,
            )
            .with_help(vec!["Inspect retained evidence before retrying.".to_owned()]),
        )
        .unwrap();
    let diagnostic = Diagnostic {
        document: DiagnosticDocument::Admitted {
            presentation: Box::new(DiagnosticPresentation::from_diagnostic(&admitted)),
            reconciliation: Some(MaterializationReconciliationPresentation {
                repository_id: "alpha".to_owned(),
                plan_digest: "sha256:plan".to_owned(),
                receipt: "exact",
                ownership: "matches-post",
            }),
        },
        class: FailureClass::EffectUnknown,
    };

    let value: serde_json::Value =
        serde_json::from_str(&diagnostic.render_json().unwrap()).unwrap();
    assert_eq!(
        value["diagnostic"]["code"],
        "materialization_outcome_unknown"
    );
    assert_eq!(value["reconciliation"]["repositoryId"], "alpha");
    assert_eq!(value["reconciliation"]["receipt"], "exact");
    assert_eq!(value["reconciliation"]["ownership"], "matches-post");
    let human = diagnostic.render_human().unwrap();
    assert!(human.contains(
        "reconciliation: repository alpha; plan sha256:plan; receipt exact; ownership matches-post"
    ));
}

#[test]
fn host_cause_projection_is_deterministic_and_deduplicates_repetition() {
    let error = anyhow!("leaf").context("middle").context("outer");
    let diagnostic = Diagnostic::from_host_error(&error);
    let value = diagnostic_value(&diagnostic);

    assert_eq!(value["summary"], "outer");
    assert_eq!(value["causes"], serde_json::json!(["middle", "leaf"]));
    assert_eq!(value["truncation"]["omitted_causes"], 0);
}

#[test]
fn host_projection_uses_api_owned_bounds_and_redaction() {
    let oversized = "x".repeat(4_200);
    let error = anyhow!(
        "request failed token=alpha password=bravo Authorization: Bearer charlie {oversized}"
    );
    let diagnostic = Diagnostic::from_host_error(&error);
    let encoded = diagnostic.render_json().unwrap();
    let value = diagnostic_value(&diagnostic);

    for secret in ["alpha", "bravo", "charlie"] {
        assert!(!encoded.contains(secret));
    }
    assert!(encoded.contains("[REDACTED]"));
    assert!(value["summary"].as_str().unwrap().chars().count() <= 4_096);
    assert!(value["truncation"]["shortened_fields"].as_u64().unwrap() > 0);
}

#[test]
fn terminal_renderer_projects_all_admitted_structured_fields() {
    let secret = format!("token=alpha {}", "x".repeat(8_192));
    let projection = DiagnosticProjection::new(
        "host_input_rejected".to_string(),
        DiagnosticStage::InputAdmission,
        DiagnosticDisposition::InvalidInput,
        "fixture".to_string(),
        DiagnosticRetry::AfterCorrection,
        DiagnosticEffectCertainty::NotApplicable,
    )
    .with_subject("record".to_string(), secret.clone())
    .with_source(
        DiagnosticProjectionSource::new(secret.clone())
            .with_position(7, Some(9))
            .with_value_path(secret.clone()),
    )
    .with_notes(vec![secret.clone()])
    .with_help(vec![secret.clone()])
    .with_facts(vec![("credential".to_string(), secret)]);
    let diagnostic = admitted_diagnostic(projection, FailureClass::InvalidInput);

    let encoded = diagnostic.render_json().unwrap();
    let human = diagnostic.render_human().unwrap();
    for rendered in [&encoded, &human] {
        assert!(!rendered.contains("alpha"));
        assert!(rendered.contains("[REDACTED]"));
    }
    for marker in [
        "subject: record",
        "--> ",
        ":7:9",
        "value path:",
        "note:",
        "credential:",
        "help:",
    ] {
        assert!(human.contains(marker), "human output omitted {marker}");
    }
    let value = diagnostic_value(&diagnostic);
    assert!(value["truncation"]["shortened_fields"].as_u64().unwrap() >= 5);
}

#[test]
fn api_dispositions_have_one_mechanical_cli_exit_policy() {
    for (disposition, class, exit) in [
        (
            DiagnosticDisposition::OperationalFailure,
            FailureClass::Operational,
            1,
        ),
        (
            DiagnosticDisposition::InvalidInput,
            FailureClass::InvalidInput,
            4,
        ),
        (DiagnosticDisposition::Blocked, FailureClass::Blocked, 5),
        (
            DiagnosticDisposition::InternalFailure,
            FailureClass::Internal,
            6,
        ),
        (
            DiagnosticDisposition::EffectOutcomeUnknown,
            FailureClass::EffectUnknown,
            7,
        ),
    ] {
        assert_eq!(class.exit_status(), exit);
        assert_eq!(class.disposition(), disposition);
    }
}

#[test]
fn initialization_fallback_has_no_invented_admitted_diagnostic_or_source_evidence() {
    let diagnostic = Diagnostic {
        document: DiagnosticDocument::InitializationFallback {
            code: "initialization_failed".to_owned(),
            summary: "façade unavailable".to_owned(),
        },
        class: FailureClass::Internal,
    };
    assert_eq!(diagnostic.exit_code(), std::process::ExitCode::from(6));
    assert_eq!(
        diagnostic.render_human().unwrap(),
        "error[initialization_failed]: façade unavailable"
    );
    let value: serde_json::Value =
        serde_json::from_str(&diagnostic.render_json().unwrap()).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"severity":"error", "code":"initialization_failed", "summary":"façade unavailable"})
    );
}
