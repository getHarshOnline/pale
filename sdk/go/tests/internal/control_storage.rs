//! Physical-inspection bridge laws, not physical-store admission tests.

use super::*;

#[test]
fn private_scope_decoder_rejects_incomplete_unknown_and_cross_scope_fields() {
    let principal = serde_json::json!({
        "scope": "principal", "runtimeStateRoot": "/tmp", "principalId": "principal-inspection",
        "logicalStoreId": "store-inspection", "controlIncarnationId": "incarnation-inspection"
    });
    for mutation in ["missing", "unknown", "cross-scope", "unknown-scope"] {
        let mut value = principal.clone();
        let object = value.as_object_mut().expect("object fixture");
        match mutation {
            "missing" => {
                object.remove("principalId");
            }
            "unknown" => {
                object.insert("extra".into(), true.into());
            }
            "cross-scope" => {
                object.insert("workspaceId".into(), "workspace-inspection".into());
            }
            _ => {
                object.insert("scope".into(), "arbitrary".into());
            }
        }
        let error = decode::<wire::LocalControlStorageInspectionRequest>(
            &serde_json::to_vec(&value).expect("fixture JSON"),
        )
        .err()
        .expect("invalid scope shape fails before API");
        assert_eq!(error.code, "invalid_binding_request");
    }
}

#[test]
fn c_export_preserves_exact_api_diagnostic_and_consumes_request_once() {
    let api = pale_sdk::PaleApi::new().expect("embedded API initializes");
    let request = serde_json::json!({
        "scope": "principal", "runtimeStateRoot": "private-relative-runtime", "principalId": "principal-inspection",
        "logicalStoreId": "store-inspection", "controlIncarnationId": "incarnation-inspection"
    });
    let expected = api
        .inspect_local_control_store(pale_sdk::LocalControlStorageInspectionRequest::principal(
            "private-relative-runtime".into(),
            "principal-inspection".into(),
            "store-inspection".into(),
            "incarnation-inspection".into(),
        ))
        .expect_err("relative root fails before host observation");
    let client = bridge_state::insert_client(bridge_state::ClientState::new(api));
    let request = bridge_state::insert_request(serde_json::to_vec(&request).expect("fixture JSON"));
    let response = pale_go_inspect_local_control_store(client, request);
    assert_eq!(
        pale_go_response_kind(response),
        bridge_state::ResponseKind::Diagnostic as u8
    );
    assert_eq!(pale_go_response_object(response), 0);
    bridge_state::with_response(response, |response| {
        assert_eq!(response.payload, expected.diagnostic().canonical_bytes());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&response.metadata).unwrap(),
            serde_json::json!({
                "diagnosticIdentity": expected.diagnostic().identity(),
                "authoringCorrections": [],
                "authoringLocation": null,
            })
        );
        assert!(!String::from_utf8_lossy(&response.metadata).contains("private-relative-runtime"));
        assert!(!String::from_utf8_lossy(&response.payload).contains("private-relative-runtime"));
    })
    .expect("response remains owned until released");
    assert!(bridge_state::take_request(request).is_none());
    pale_go_response_drop(response);
    pale_go_client_drop(client);
}

#[test]
fn go_projection_fixture_is_exact_api_owned_serialization() {
    use pale_sdk::{
        LocalControlStorageInspectionRejection as Rejection,
        LocalControlStorageInspectionState as State,
        LocalControlStorageProfileRequirement as Requirement, LocalControlStoreKind as Kind,
    };
    let fixtures: Vec<serde_json::Value> = serde_json::from_slice(include_bytes!(
        "../fixtures/control-storage-projection.json"
    ))
    .expect("fixture JSON");
    let expected = vec![
        State::Missing,
        State::Empty,
        State::Ready,
        State::ProfileUnselected {
            unresolved: vec![
                Requirement::BackupRestorePolicy,
                Requirement::AuthorityCompactionPolicy,
                Requirement::AuthorityRetentionPolicy,
                Requirement::AuthorityCheckpointCadence,
                Requirement::AuthorityMaximumTailEntries,
                Requirement::AuthorityMaximumCheckpointCandidates,
                Requirement::AuthorityMaximumCheckpointBytes,
                Requirement::AuthorityMaximumEntryBytes,
                Requirement::AuthorityCorruptionChecksumBytes,
                Requirement::AuthorityCorruptionChecksum,
                Requirement::AuthorityFrameEncoding,
                Requirement::SqliteDurabilityGrouping,
                Requirement::SqliteWalAutocheckpointFrames,
                Requirement::SqliteCheckpointMode,
                Requirement::SqliteCheckpointFullFsync,
                Requirement::SqliteFullFsync,
                Requirement::SqliteSynchronous,
                Requirement::SqliteCompileOptions,
            ],
        },
        State::ProfileUnselected { unresolved: vec![] },
        State::Rejected(Rejection::UnsafeEntryKind),
        State::Rejected(Rejection::CorruptOrPartial),
        State::Rejected(Rejection::WrongStoreKind),
        State::Rejected(Rejection::WrongLogicalStoreId),
        State::Rejected(Rejection::WrongControlIncarnation),
        State::Rejected(Rejection::WrongPrincipalScope),
        State::Rejected(Rejection::WrongDeploymentScope),
        State::Rejected(Rejection::SourceIdentityMismatch),
        State::Rejected(Rejection::NewerVersion {
            observed: 9,
            supported: 1,
        }),
        State::Rejected(Rejection::SqliteBuildMismatch),
        State::Rejected(Rejection::StructuralProfileMismatch),
    ];
    assert_eq!(fixtures.len(), expected.len());
    for (index, (fixture, state)) in fixtures.iter().zip(expected).enumerate() {
        let kind = if index % 2 == 0 {
            Kind::PrincipalTrust
        } else {
            Kind::DeploymentControl
        };
        assert_eq!(
            fixture["kind"],
            serde_json::to_value(kind).expect("kind serialization")
        );
        assert_eq!(
            fixture["state"],
            serde_json::to_value(state).expect("state serialization")
        );
    }
}
