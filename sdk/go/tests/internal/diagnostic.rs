//! Actual compiler evidence across the private response boundary.

use super::*;
use base64::Engine as _;

fn compiler_request(contract_rejection: bool) -> Vec<u8> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../interfaces/lsp/tests/fixtures/authoring/visibility-widening");
    let documents: Vec<_> = [
        "pale-workspace.toml",
        "module.toml",
        "consumer-module.toml",
        "alpha.toml",
        "beta.toml",
        "consumer.toml",
        "resolution.lock.toml",
        "universe.lock.toml",
    ]
    .into_iter()
    .map(|label| {
        let mut text = std::fs::read_to_string(root.join(label)).unwrap();
        if contract_rejection && label == "pale-workspace.toml" {
            text = text.replacen("schema_version = 1", "schema_version = 2", 1);
        }
        serde_json::json!({
            "label": label,
            "bytes": base64::engine::general_purpose::STANDARD.encode(text),
        })
    })
    .collect();
    serde_json::to_vec(&serde_json::json!({
        "rootManifest": "pale-workspace.toml", "documents": documents,
    }))
    .unwrap()
}

#[test]
fn compiler_sidecars_preserve_exact_sdk_identity_payload_and_response_custody() {
    for contract_rejection in [false, true] {
        let request = compiler_request(contract_rejection);
        let api = pale_sdk::PaleApi::new().unwrap();
        let failure = api
            .compile_portable_workspace(
                decode::<wire::PortableRequest>(&request)
                    .unwrap()
                    .into_api()
                    .unwrap(),
            )
            .expect_err("the actual compiler rejects this fixture");
        assert_eq!(
            failure.authoring_corrections().len(),
            usize::from(!contract_rejection)
        );
        assert!(failure.authoring_location().is_some());
        for correction in failure.authoring_corrections() {
            assert_eq!(
                correction.diagnostic_identity(),
                failure.diagnostic().identity()
            );
        }
        let expected_metadata = serde_json::json!({
            "diagnosticIdentity": failure.diagnostic().identity(),
            "authoringCorrections": failure.authoring_corrections(),
            "authoringLocation": failure.authoring_location(),
        });
        let initialized = pale_go_client_new();
        assert_eq!(
            pale_go_response_kind(initialized),
            ResponseKind::Success as u8
        );
        let client = pale_go_response_object(initialized);
        pale_go_response_drop(initialized);
        let request = bridge_state::insert_request(request);
        let response = pale_go_compile_portable_workspace(client, request);
        assert_eq!(
            pale_go_response_kind(response),
            ResponseKind::Diagnostic as u8
        );
        assert_eq!(pale_go_response_object(response), 0);
        assert!(bridge_state::take_request(request).is_none());
        // A diagnostic response owns copies independently of its client.
        pale_go_client_drop(client);
        let (metadata, payload) = bridge_state::with_response(response, |response| {
            assert!(response.pending_values.is_none());
            (response.metadata.clone(), response.payload.clone())
        })
        .unwrap();
        pale_go_response_drop(response);
        assert!(bridge_state::with_response(response, |_| ()).is_none());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&metadata).unwrap(),
            expected_metadata
        );
        assert_eq!(payload, failure.diagnostic().canonical_bytes());
    }
}
