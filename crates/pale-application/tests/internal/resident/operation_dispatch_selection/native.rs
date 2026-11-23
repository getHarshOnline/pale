//! Native selection reuses admitted starts and the actual sealed action registry.
//! The start fixture models signing and custody; these tests never execute a call.

use super::*;
use crate::actions::{NativeActionRegistry, PreparedNativeAction};
use crate::local_operation_start_fixture::RecoveryParents;

pub(super) fn input() -> serde_json::Value {
    serde_json::json!({"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs"]}})
}

fn with_native(case: &str, run: impl FnOnce(AttemptSelectionContext<'_>, RecoveryParents<'_>)) {
    let (compiled, materializations, profile) = source_generation_inputs_with(|_| {}, |_| {});
    let generation = AdmittedCompiledGeneration::admit(
        compiled,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .unwrap();
    local_start::with_issued_request(
        &generation,
        &profile,
        "native-selection",
        |request| {
            let descriptor = pale_contracts::schema_descriptor("action-input.v1").unwrap();
            request.body.schema_id = SchemaId::parse(descriptor.schema_id).unwrap();
            request.body.schema_sha256 = SchemaDigest::parse(descriptor.schema_sha256).unwrap();
            request.body.payload = input();
            match case {
                "wrong-body-schema" => {
                    request.body.schema_id = SchemaId::parse("other-input.v1").unwrap()
                }
                "wrong-body-schema-digest" => {
                    request.body.schema_sha256 = SchemaDigest::parse("0".repeat(64)).unwrap()
                }
                "unknown-input-field" => request.body.payload["extra"] = serde_json::json!(true),
                "wrong-input-kind" => {
                    request.body.payload["kind"] = serde_json::json!("weighted_ranking")
                }
                _ => {}
            }
            request.body.payload_sha256 = ArtifactDigest::hash_bytes(
                canon::to_canonical_string(&request.body.payload)
                    .unwrap()
                    .as_bytes(),
            );
        },
        |operation, request, parents| {
            run(
                AttemptSelectionContext {
                    operation,
                    request_ref: operation.request_coordinate().record_ref(),
                    request,
                    session: parents.session.lease(),
                    lease: parents.lease,
                    generation: &generation,
                },
                parents,
            );
        },
    );
}

pub(super) fn with_admission<R>(
    parents: AttemptSelectionContext<'_>,
    signing: &SigningAuthorityBindingV1,
    run: impl FnOnce(&CapabilityAttemptIdV1, CapabilitySuccessAdmissionContextV1<'_>) -> R,
) -> R {
    let attempt = CapabilityAttemptIdV1::parse("native-attempt-1").unwrap();
    let binding = parents.request.local_request_binding().unwrap();
    let contracts = crate::session_admission_fixture::local_start_contract_set();
    let schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
    let kind = BoundedResidentTextV1::parse(CapabilityProvider::NativeRust.as_str()).unwrap();
    let registry = NativeActionRegistry::new().unwrap();
    let version = BoundedResidentTextV1::parse(
        registry
            .contract(PATH_INVENTORY_ACTION_ID)
            .unwrap()
            .version
            .to_string(),
    )
    .unwrap();
    run(
        &attempt,
        CapabilitySuccessAdmissionContextV1 {
            result: CapabilityResultAdmissionContextV1 {
                expected_schema_sha256: &schema,
                expected_request_binding: &binding,
                expected_attempt_id: &attempt,
                expected_implementation_kind: &kind,
                expected_implementation_version: &version,
                signing_authority: signing,
            },
            contract_set_ref: binding.contract_set_ref(),
            contract_set: &contracts,
        },
    )
}

fn rejects(result: Result<(AttemptSelection, PreparedNativeAction<'_>), AttemptSelectionError>) {
    assert!(
        result.is_err(),
        "candidate must reject before returning a prepared native call"
    );
}

#[test]
fn native_selection_retains_actual_contract_request_and_result_schema_without_a_provider() {
    with_native("exact", |parents, authority| {
        with_admission(parents, authority.signing, |attempt, admission| {
            let registry = NativeActionRegistry::new().unwrap();
            let (selection, call) =
                AttemptSelection::capture_native(parents, &registry, attempt, admission).unwrap();
            assert!(std::ptr::eq(
                call.contract(),
                registry.contract(PATH_INVENTORY_ACTION_ID).unwrap()
            ));
            assert_eq!(
                serde_json::to_value(&call.request().input).unwrap(),
                parents.request.body().payload
            );
            assert_eq!(call.request().action_id, PATH_INVENTORY_ACTION_ID);
            assert_eq!(
                call.request().contract_digest,
                call.contract().contract_digest
            );
            assert!(selection.provider().is_none());
            assert!(selection.provider_request(parents).is_err());
            assert_eq!(
                selection.implementation_kind().as_str(),
                CapabilityProvider::NativeRust.as_str()
            );
            assert_eq!(
                selection.implementation_version().as_str(),
                call.contract().version.to_string()
            );
            assert_eq!(selection.result_authority(), authority.signing);
            assert_eq!(
                selection.result_schema(),
                (
                    &SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA).unwrap(),
                    &schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID)
                )
            );
            let execution = pale_contracts::record_schema_identity("action-execution.v1").unwrap();
            assert_eq!(
                selection.native_payload_schema(),
                Some((&execution.schema, &execution.schema_sha256))
            );
            let encoded = serde_json::to_value(&selection).unwrap();
            assert_eq!(encoded["implementation"]["kind"], "native_rust");
            assert_eq!(encoded["implementation"]["action_id"], call.contract().id);
            assert_eq!(
                encoded["implementation"]["action_version"],
                call.contract().version
            );
            assert_eq!(
                encoded["implementation"]["contract_digest"],
                serde_json::to_value(&call.contract().contract_digest).unwrap()
            );
            assert_eq!(
                encoded["implementation"]["request_digest"],
                serde_json::to_value(&call.request().request_digest).unwrap()
            );
            assert!(encoded["implementation"].get("provider").is_none());
            let record = selection
                .record(record_ref("native-attempt"), 65_536)
                .unwrap();
            let length = record.canonical_record_jcs().as_slice().len();
            let decoded = AttemptSelection::decode(&record, &evidence(&record), length).unwrap();
            assert_eq!(decoded, selection);
            decoded.validate(&parents).unwrap();
            assert!(matches!(
                AttemptSelection::decode(&record, &evidence(&record), length - 1).unwrap_err(),
                AttemptSelectionError::BoundExceeded
            ));
            assert!(matches!(
                selection
                    .record(record_ref("native-attempt"), length - 1)
                    .unwrap_err(),
                AttemptSelectionError::BoundExceeded
            ));
            drop(call);
        })
    });
}

#[test]
fn native_capture_rejects_unselected_body_schema_or_malformed_action_input() {
    for case in [
        "wrong-body-schema",
        "wrong-body-schema-digest",
        "unknown-input-field",
        "wrong-input-kind",
    ] {
        with_native(case, |parents, authority| {
            with_admission(parents, authority.signing, |attempt, admission| {
                let registry = NativeActionRegistry::new().unwrap();
                let result =
                    AttemptSelection::capture_native(parents, &registry, attempt, admission);
                if matches!(case, "wrong-body-schema" | "wrong-body-schema-digest") {
                    assert!(matches!(result, Err(AttemptSelectionError::Association)));
                } else {
                    assert!(matches!(
                        result,
                        Err(AttemptSelectionError::NativeAction(
                            crate::actions::NativeActionInvocationError::MalformedInput(_)
                        ))
                    ));
                }
            })
        });
    }
}

#[test]
fn native_capture_joins_independent_result_context_and_original_issued_coordinates() {
    with_native("exact", |parents, authority| {
        with_admission(parents, authority.signing, |attempt, admission| {
            let registry = NativeActionRegistry::new().unwrap();
            let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
            let version = BoundedResidentTextV1::parse("999").unwrap();
            let schema = SchemaDigest::parse("0".repeat(64)).unwrap();
            let other_attempt = CapabilityAttemptIdV1::parse("other-attempt").unwrap();
            let other_reference = record_ref("other-contract-set");
            let mut request_binding =
                serde_json::to_value(admission.result.expected_request_binding).unwrap();
            request_binding["request_id"] = serde_json::json!("other-request");
            let request_binding: RequestBindingV1 =
                serde_json::from_value(request_binding).unwrap();
            for change in 0..6 {
                let mut changed = admission;
                match change {
                    0 => changed.result.expected_implementation_kind = &kind,
                    1 => changed.result.expected_implementation_version = &version,
                    2 => changed.result.expected_schema_sha256 = &schema,
                    3 => changed.result.expected_attempt_id = &other_attempt,
                    4 => changed.contract_set_ref = &other_reference,
                    5 => changed.result.expected_request_binding = &request_binding,
                    _ => unreachable!(),
                }
                rejects(AttemptSelection::capture_native(
                    parents, &registry, attempt, changed,
                ));
            }
            let alias = record_ref("same-byte-request-alias");
            rejects(AttemptSelection::capture_native(
                AttemptSelectionContext {
                    request_ref: &alias,
                    ..parents
                },
                &registry,
                attempt,
                admission,
            ));
            let (other_generation, _) = host_generation("bounded_compute");
            rejects(AttemptSelection::capture_native(
                AttemptSelectionContext {
                    generation: &other_generation,
                    ..parents
                },
                &registry,
                attempt,
                admission,
            ));
        })
    });
}

#[test]
fn private_codec_rejects_hybrid_implementation_shapes() {
    with_native("exact", |parents, authority| {
        with_admission(parents, authority.signing, |attempt, admission| {
            let registry = NativeActionRegistry::new().unwrap();
            let (selection, _) =
                AttemptSelection::capture_native(parents, &registry, attempt, admission).unwrap();
            let record = selection
                .record(record_ref("native-attempt"), 65_536)
                .unwrap();
            let mut hybrid = serde_json::to_value(selection).unwrap();
            hybrid["implementation"]["provider"] =
                serde_json::json!({"provider_binding_id":"invented"});
            let altered = replace_bytes(
                &record,
                canon::to_canonical_string(&hybrid).unwrap().into_bytes(),
            );
            assert!(matches!(
                AttemptSelection::decode(&altered, &evidence(&altered), 65_536).unwrap_err(),
                AttemptSelectionError::Encoding
            ));
        })
    });
}
