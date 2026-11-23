//! Exact SDK evidence through the shared command presentation.

use super::*;
use pale_sdk::{PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest};

use crate::authoring_failure_fixture as fixture;

#[test]
fn rejected_compilation_keeps_exact_evidence_after_its_error_is_dropped() {
    for contract_rejection in [false, true] {
        let mut documents = fixture::documents();
        if contract_rejection {
            documents[0].1 = documents[0]
                .1
                .replace("schema_version = 1", "schema_version = 2");
        }
        let api = PaleApi::new().unwrap();
        let error = api
            .compile_portable_workspace(PortableWorkspaceRequest::new(
                "pale-workspace.toml".to_owned(),
                documents
                    .into_iter()
                    .map(|(label, content)| {
                        PortableWorkspaceDocument::new(label.to_owned(), content.into_bytes())
                    })
                    .collect(),
            ))
            .expect_err("the actual compiler rejects these documents");
        assert_eq!(
            error.authoring_corrections().len(),
            usize::from(!contract_rejection)
        );
        assert!(error.authoring_location().is_some());
        let canonical = error.diagnostic().canonical_bytes().to_vec();
        let expected = serde_json::json!({
            "diagnostic": serde_json::from_slice::<serde_json::Value>(&canonical).unwrap(),
            "diagnosticIdentity": error.diagnostic().identity(),
            "authoringCorrections": error.authoring_corrections(),
            "authoringLocation": error.authoring_location(),
        });
        let presentation = DiagnosticPresentation::from_api_error(&error);
        drop(error);
        drop(api);
        assert_eq!(presentation.diagnostic().canonical_bytes(), canonical);
        let text = presentation.to_json().unwrap();
        assert_eq!(text, serde_json::to_string_pretty(&expected).unwrap());
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value, expected);
        assert_eq!(value.as_object().unwrap().len(), 4);
        assert_eq!(
            value["diagnosticIdentity"],
            presentation.diagnostic().identity().as_str()
        );
        let readmitted = PaleApi::new()
            .unwrap()
            .admit_diagnostic(&canonical)
            .unwrap();
        assert_eq!(readmitted.identity(), presentation.diagnostic().identity());
        if !contract_rejection {
            assert_eq!(
                value["authoringCorrections"][0],
                serde_json::json!({
                    "kind": "narrow-import-visibility",
                    "source_label": "consumer-module.toml",
                    "path": [{"kind":"field", "value":"module"}, {"kind":"field", "value":"imports"}, {"kind":"index", "value":0}, {"kind":"field", "value":"visibility_ceiling"}],
                    "expected_source": "\"public\"",
                    "replacement_source": "\"protected\"",
                    "proof_subject": {"kind":"export", "value":"candidate::public-api", "authored_value":"public-api"},
                    "diagnostic_identity": value["diagnosticIdentity"],
                })
            );
        }
    }
}

#[test]
fn unrelated_and_diagnostic_only_results_have_explicitly_absent_source_evidence() {
    let error = PaleApi::new()
        .unwrap()
        .contract_schema("not-a-schema")
        .unwrap_err();
    let from_error = DiagnosticPresentation::from_api_error(&error);
    let from_diagnostic = DiagnosticPresentation::from_diagnostic(error.diagnostic());
    assert_eq!(
        from_error.to_json().unwrap(),
        from_diagnostic.to_json().unwrap()
    );
    assert!(from_error.authoring_corrections().is_empty());
    assert!(from_error.authoring_location().is_none());
    let value: serde_json::Value = serde_json::from_str(&from_error.to_json().unwrap()).unwrap();
    assert_eq!(value["authoringCorrections"], serde_json::json!([]));
    assert!(value["authoringLocation"].is_null());
    assert_eq!(
        value["diagnosticIdentity"],
        error.diagnostic().identity().as_str()
    );
}
