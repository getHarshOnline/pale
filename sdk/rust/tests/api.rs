//! Public-boundary tests for the owned initial-v1 façade.

use std::error::Error as _;

use pale_sdk::{
    ContractDocument, DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticProjection,
    DiagnosticProjectionSource, DiagnosticProjector, DiagnosticRetry, DiagnosticStage,
    NativeActionPayloadRequest, NativeActionRequest, PaleApi, RepositoryNativeActionCatalogRequest,
    WorkspaceModuleCandidate,
};

const DIAGNOSTIC_ID: &str = "diagnostic.v1";
const PATH_INVENTORY_ID: &str = "pale.compute.path-inventory";

fn api() -> PaleApi {
    PaleApi::new().expect("embedded API dependencies are coherent")
}

fn valid_diagnostic_value() -> serde_json::Value {
    let identity = pale_contracts::record_schema_identity(DIAGNOSTIC_ID)
        .expect("diagnostic identity is embedded");
    let capabilities = pale_contracts::schema_descriptor("capabilities.v1")
        .expect("capabilities descriptor is embedded");
    serde_json::json!({
        "schema": identity.schema.as_str(),
        "schema_sha256": identity.schema_sha256.as_str(),
        "schema_version": 1,
        "code": "workspace.binding_missing",
        "stage": "semantic_admission",
        "severity": "error",
        "disposition": "invalid_input",
        "summary": "one selected binding is missing",
        "subject": {"kind": "operation", "id": "compile"},
        "source": {
            "label": "workspace/profile.toml",
            "line": 4,
            "column": 7,
            "byte_start": 18,
            "byte_end": 25,
            "value_path": "operation_bindings[0].provider"
        },
        "schema_coordinate": {
            "registry_id": "capabilities.v1",
            "schema_sha256": capabilities.schema_sha256,
            "schema_path": "/properties/operation_bindings"
        },
        "facts": [{"key": "binding", "value": "compile"}],
        "retry": "after_correction",
        "effect": "not_applicable",
        "truncation": {
            "shortened_fields": 0,
            "omitted_causes": 0,
            "omitted_notes": 0,
            "omitted_help": 0,
            "omitted_facts": 0
        }
    })
}

fn canonical(value: &serde_json::Value) -> Vec<u8> {
    pale_core::canon::to_canonical_string(value)
        .expect("test value follows the canonical record grammar")
        .into_bytes()
}

#[path = "support/repository_catalog.rs"]
mod repository_catalog;
use repository_catalog::repository_catalog_fixture;

fn assert_policy(
    diagnostic: &pale_sdk::DiagnosticV1,
    stage: DiagnosticStage,
    disposition: DiagnosticDisposition,
    retry: DiagnosticRetry,
) {
    assert_eq!(diagnostic.stage(), stage);
    assert_eq!(diagnostic.disposition(), disposition);
    assert_eq!(diagnostic.retry(), retry);
    assert_eq!(
        diagnostic.effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
}

#[test]
fn diagnostic_admission_is_strict_structural_typed_and_semantic() {
    let api = api();
    let value = valid_diagnostic_value();
    let admitted = api
        .admit_diagnostic(&canonical(&value))
        .expect("valid diagnostic is admitted");

    assert_eq!(admitted.code(), "workspace.binding_missing");
    assert_eq!(admitted.stage(), DiagnosticStage::SemanticAdmission);
    assert_eq!(admitted.disposition(), DiagnosticDisposition::InvalidInput);
    assert_eq!(admitted.summary(), "one selected binding is missing");
    assert_eq!(admitted.canonical_bytes(), canonical(&value));

    let duplicate = br#"{"schema":1,"schema":1}"#;
    let error = api
        .admit_diagnostic(duplicate)
        .expect_err("duplicate keys fail at strict ingress");
    assert_eq!(error.diagnostic().code(), "invalid_json");

    let mut unknown_field = valid_diagnostic_value();
    unknown_field["unexpected"] = serde_json::json!(true);
    let error = api
        .admit_diagnostic(&canonical(&unknown_field))
        .expect_err("unknown fields fail generated-schema admission");
    assert_eq!(error.diagnostic().code(), "diagnostic_contract_rejected");
    let rejection = pale_core::canon::parse_strict(error.diagnostic().canonical_bytes())
        .expect("contract rejection remains an admitted diagnostic");
    assert_eq!(rejection["source"]["label"], "input");
    assert!(rejection["source"]["value_path"].is_string());
    assert!(rejection["schema_coordinate"]["schema_path"].is_string());
    let fact_keys = rejection["facts"]
        .as_array()
        .expect("contract rejection carries presentation facts")
        .iter()
        .map(|fact| fact["key"].as_str().expect("fact key"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        fact_keys,
        std::collections::BTreeSet::from([
            "omitted_violations",
            "retained_violations",
            "schema_expectation",
            "schema_keyword",
        ])
    );

    let mut unsafe_source = valid_diagnostic_value();
    unsafe_source["source"]["label"] = serde_json::json!("/private/host/path");
    let error = api
        .admit_diagnostic(&canonical(&unsafe_source))
        .expect_err("absolute source labels fail semantic admission");
    assert_eq!(error.diagnostic().code(), "diagnostic_semantic_rejected");

    let mut invalid_schema_path = valid_diagnostic_value();
    invalid_schema_path["schema_coordinate"]["schema_path"] = serde_json::json!("properties/x");
    assert_eq!(
        api.admit_diagnostic(&canonical(&invalid_schema_path))
            .expect_err("schema paths are JSON Pointer-specific")
            .diagnostic()
            .code(),
        "diagnostic_semantic_rejected"
    );

    let mut effect_mismatch = valid_diagnostic_value();
    effect_mismatch["effect"] = serde_json::json!("outcome_unknown");
    assert_eq!(
        api.admit_diagnostic(&canonical(&effect_mismatch))
            .expect_err("effect-unknown state is admitted as one coherent tuple")
            .diagnostic()
            .code(),
        "diagnostic_semantic_rejected"
    );

    let mut duplicate_fact = valid_diagnostic_value();
    duplicate_fact["facts"] = serde_json::json!([
        {"key": "binding", "value": "first"},
        {"key": "binding", "value": "second"}
    ]);
    assert_eq!(
        api.admit_diagnostic(&canonical(&duplicate_fact))
            .expect_err("fact keys are a unique presentation map")
            .diagnostic()
            .code(),
        "diagnostic_semantic_rejected"
    );
}

#[test]
fn owner_projection_has_one_redaction_bounding_and_admission_kernel() {
    let api = api();
    let projection = DiagnosticProjection::new(
        "workspace.binding_missing".to_string(),
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::InvalidInput,
        format!("binding token=private {}", "x".repeat(4_200)),
        DiagnosticRetry::AfterCorrection,
        DiagnosticEffectCertainty::NotApplicable,
    )
    .with_subject("operation".to_string(), "compile".to_string())
    .with_source(
        DiagnosticProjectionSource::new("workspace/profile.toml".to_string())
            .with_position(4, Some(7))
            .with_value_path("operation_bindings[0].provider".to_string()),
    )
    .with_schema_reference(
        "capabilities.v1".to_string(),
        Some("/properties/operation_bindings".to_string()),
    )
    .with_notes((0..19).map(|index| format!("note-{index}")).collect())
    .with_help((0..21).map(|index| format!("help-{index}")).collect())
    .with_facts(
        (0..23)
            .map(|index| (format!("fact_{index}"), format!("value-{index}")))
            .collect(),
    );

    let diagnostic = api
        .project_diagnostic(projection)
        .expect("the typed projection is admitted");
    let value = pale_core::canon::parse_strict(diagnostic.canonical_bytes()).unwrap();
    pale_contracts::validate_value(&value, DIAGNOSTIC_ID).unwrap();
    assert!(!diagnostic.canonical_json().contains("private"));
    assert!(diagnostic.canonical_json().contains("[REDACTED]"));
    assert_eq!(value["notes"].as_array().unwrap().len(), 16);
    assert_eq!(value["help"].as_array().unwrap().len(), 16);
    assert_eq!(value["facts"].as_array().unwrap().len(), 16);
    assert_eq!(value["truncation"]["omitted_notes"], 3);
    assert_eq!(value["truncation"]["omitted_help"], 5);
    assert_eq!(value["truncation"]["omitted_facts"], 7);
    assert_eq!(
        value["schema_coordinate"]["schema_sha256"],
        pale_contracts::schema_descriptor("capabilities.v1")
            .unwrap()
            .schema_sha256
    );

    let mismatch = DiagnosticProjection::new(
        "effect_state".to_string(),
        DiagnosticStage::Materialization,
        DiagnosticDisposition::EffectOutcomeUnknown,
        "effect outcome is unknown".to_string(),
        DiagnosticRetry::NotIndicated,
        DiagnosticEffectCertainty::OutcomeUnknown,
    );
    assert_eq!(
        api.project_diagnostic(mismatch)
            .expect_err("effect-unknown is one coherent policy tuple")
            .diagnostic()
            .code(),
        "diagnostic_projection_rejected"
    );

    let durable_ambiguity = DiagnosticProjection::new(
        "journal_commit_ambiguous".to_string(),
        DiagnosticStage::ResidentRecovery,
        DiagnosticDisposition::Blocked,
        "durable journal state must be reconciled before retry".to_string(),
        DiagnosticRetry::ReconcileFirst,
        DiagnosticEffectCertainty::NotApplicable,
    );
    let durable_ambiguity = api
        .project_diagnostic(durable_ambiguity)
        .expect("durable ambiguity need not imply an unknown external effect");
    assert_eq!(durable_ambiguity.stage(), DiagnosticStage::ResidentRecovery);
    assert_eq!(durable_ambiguity.retry(), DiagnosticRetry::ReconcileFirst);
    assert_eq!(
        durable_ambiguity.effect(),
        DiagnosticEffectCertainty::NotApplicable
    );

    let unsafe_source = DiagnosticProjection::new(
        "input".to_string(),
        DiagnosticStage::InputAdmission,
        DiagnosticDisposition::InvalidInput,
        "input is invalid".to_string(),
        DiagnosticRetry::AfterCorrection,
        DiagnosticEffectCertainty::NotApplicable,
    )
    .with_source(DiagnosticProjectionSource::new(
        "/private/input.toml".to_string(),
    ));
    assert_eq!(
        api.project_diagnostic(unsafe_source)
            .expect_err("host-root authority cannot enter a portable diagnostic")
            .diagnostic()
            .code(),
        "diagnostic_projection_rejected"
    );

    let unknown_schema = DiagnosticProjection::new(
        "input".to_string(),
        DiagnosticStage::InputAdmission,
        DiagnosticDisposition::InvalidInput,
        "input is invalid".to_string(),
        DiagnosticRetry::AfterCorrection,
        DiagnosticEffectCertainty::NotApplicable,
    )
    .with_schema_reference("not-registered.v1".to_string(), None);
    let error = api
        .project_diagnostic(unknown_schema)
        .expect_err("a caller-supplied unknown schema identity is invalid input");
    assert_eq!(error.diagnostic().code(), "diagnostic_projection_rejected");
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
}

#[test]
fn narrow_diagnostic_projector_matches_the_complete_facade() {
    fn candidate() -> DiagnosticProjection {
        DiagnosticProjection::new(
            "protocol_input_rejected".to_string(),
            DiagnosticStage::InputAdmission,
            DiagnosticDisposition::InvalidInput,
            "protocol input is invalid".to_string(),
            DiagnosticRetry::AfterCorrection,
            DiagnosticEffectCertainty::NotApplicable,
        )
        .with_facts(vec![(
            "protocol_session_state".to_string(),
            "disconnected".to_string(),
        )])
    }

    let broad = api()
        .project_diagnostic(candidate())
        .expect("the complete facade projects the candidate");
    let narrow = DiagnosticProjector::new()
        .expect("the embedded diagnostic contract is coherent")
        .project(candidate())
        .expect("the narrow projector uses the same admission kernel");

    assert_eq!(narrow.identity(), broad.identity());
    assert_eq!(narrow.canonical_bytes(), broad.canonical_bytes());

    fn unsafe_candidate() -> DiagnosticProjection {
        candidate().with_source(DiagnosticProjectionSource::new(
            "/private/host/path".to_string(),
        ))
    }
    let broad_error = api()
        .project_diagnostic(unsafe_candidate())
        .expect_err("the complete facade rejects a host-absolute source");
    let narrow_error = DiagnosticProjector::new()
        .expect("the embedded diagnostic contract is coherent")
        .project(unsafe_candidate())
        .expect_err("the narrow projector enforces the same source boundary");
    assert_eq!(
        narrow_error.diagnostic().canonical_bytes(),
        broad_error.diagnostic().canonical_bytes()
    );
}

#[test]
fn contract_facade_returns_owned_exact_current_bundle_data() {
    let api = api();
    let bundle = api.contract_bundle_identity();
    let manifest =
        pale_contracts::schema_registry_identity().expect("embedded contract registry is coherent");
    assert_eq!(
        bundle.member_count(),
        u32::try_from(pale_contracts::schema_count().unwrap()).unwrap()
    );
    assert_eq!(
        bundle.schema_manifest_sha256(),
        manifest.schema_manifest_sha256.as_str()
    );
    assert_eq!(
        bundle.schema_set_sha256(),
        manifest.schema_set_sha256.as_str()
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(bundle.canonical_bytes()).unwrap(),
        serde_json::json!({
            "memberCount": bundle.member_count(),
            "schemaManifestSha256": bundle.schema_manifest_sha256(),
            "schemaSetSha256": bundle.schema_set_sha256(),
        })
    );
    assert_eq!(
        bundle.canonical_bytes(),
        canonical(&serde_json::from_slice(bundle.canonical_bytes()).unwrap())
    );

    let schema = api
        .contract_schema(DIAGNOSTIC_ID)
        .expect("diagnostic schema is available through the façade");
    assert_eq!(schema.registry_id(), DIAGNOSTIC_ID);
    assert_eq!(schema.wire_schema_id(), Some("pale/diagnostic.v1"));
    assert_eq!(
        pale_core::sha256_hex(schema.bytes()),
        schema.schema_sha256()
    );
    assert_eq!(
        schema.bytes(),
        pale_contracts::embedded_schema(DIAGNOSTIC_ID)
            .expect("schema is embedded")
            .as_bytes()
    );

    let value = valid_diagnostic_value();
    let validated = api
        .validate_contract_document(ContractDocument::new(
            DIAGNOSTIC_ID.to_string(),
            canonical(&value),
        ))
        .expect("the structural façade admits the generated shape");
    assert_eq!(validated.schema_id(), DIAGNOSTIC_ID);
    assert_eq!(validated.schema_sha256(), schema.schema_sha256());
    assert_eq!(validated.canonical_bytes(), canonical(&value));

    let error = api
        .contract_schema("unknown.v1")
        .expect_err("unknown schemas remain structured failures");
    assert_eq!(error.diagnostic().code(), "contract_rejected");
    assert!(error.source().is_some());

    let private = api
        .contract_schema("capabilities.v1")
        .expect_err("embedded producers remain non-facade by default");
    assert_eq!(private.diagnostic().code(), "contract_rejected");
    assert_eq!(private.diagnostic().summary(), error.diagnostic().summary());
}

#[test]
fn workspace_module_builder_uses_the_typed_sealing_and_schema_kernel() {
    let api = api();
    let expected = include_bytes!("fixtures/workspace-language/populated/module-candidate.json");
    let expected_value: serde_json::Value =
        serde_json::from_slice(expected).expect("workspace-module fixture is JSON");
    let preimage = pale_core::canon::to_canonical_string(&expected_value["module"])
        .expect("fixture preimage follows canonical JSON")
        .into_bytes();

    let built = api
        .build_workspace_module(WorkspaceModuleCandidate::new(preimage))
        .expect("typed construction seals the fixture preimage");
    assert_eq!(built.schema_id(), "workspace-module.v1");
    assert_eq!(
        built.schema_sha256(),
        pale_contracts::schema_descriptor("workspace-module.v1")
            .expect("workspace-module schema is embedded")
            .schema_sha256
    );
    assert_eq!(
        built.canonical_bytes(),
        canonical(&expected_value),
        "the builder and direct authored record must have one canonical result"
    );

    let duplicate = br#"{"module_id":"first","module_id":"second"}"#.to_vec();
    let error = api
        .build_workspace_module(WorkspaceModuleCandidate::new(duplicate))
        .expect_err("duplicate keys fail at strict ingress");
    assert_eq!(
        error.diagnostic().code(),
        "workspace_module_candidate_rejected"
    );

    let mut malformed = expected_value["module"].clone();
    malformed["ambient_path"] = serde_json::json!("/private/workspace");
    let error = api
        .build_workspace_module(WorkspaceModuleCandidate::new(canonical(&malformed)))
        .expect_err("unknown fields cannot enter the typed preimage");
    assert_eq!(
        error.diagnostic().code(),
        "workspace_module_candidate_rejected"
    );
    assert!(!String::from_utf8_lossy(error.diagnostic().canonical_bytes()).contains("/private"));

    api.contract_schema("workspace-module.v1")
        .expect_err("the specialized builder does not widen general schema lookup");
}

#[test]
fn native_action_facade_is_pure_canonical_and_redaction_safe() {
    let api = api();
    let request = NativeActionRequest::new(
        PATH_INVENTORY_ID.to_string(),
        br#"{
            "kind":"path_inventory",
            "payload":{"style":"posix_relative","paths":["src/./lib.rs"]}
        }"#
        .to_vec(),
    );
    let execution = api
        .invoke_native_action(request)
        .expect("the compiled pure path inventory executes");
    assert_eq!(execution.action_id(), PATH_INVENTORY_ID);
    assert_eq!(execution.request_digest().len(), 64);
    assert_eq!(execution.schema_sha256().len(), 64);
    let value = pale_core::canon::parse_strict(execution.canonical_bytes())
        .expect("execution bytes remain strict JSON");
    pale_contracts::validate_value(&value, "action-execution.v1")
        .expect("execution bytes retain their generated contract");

    let secret = "super-private-value";
    let error = api
        .invoke_native_action(NativeActionRequest::new(
            format!("pale.compute.unknown?token={secret}"),
            b"null".to_vec(),
        ))
        .expect_err("unknown actions fail without an effect");
    let diagnostic = error.diagnostic().canonical_bytes();
    assert!(!String::from_utf8_lossy(diagnostic).contains(secret));
    assert!(!format!("{error:?}").contains(secret));
    assert!(error.source().is_some());
}

#[test]
fn native_action_descriptors_and_payload_admission_share_one_registry() {
    let api = api();
    let descriptors = api
        .native_action_descriptors()
        .expect("the sealed native-action catalog projects canonically");
    assert!(!descriptors.is_empty());
    assert!(descriptors
        .windows(2)
        .all(|pair| pair[0].action_id() < pair[1].action_id()));

    let descriptor = descriptors
        .iter()
        .find(|descriptor| descriptor.action_id() == PATH_INVENTORY_ID)
        .expect("path inventory is exposed by the sealed catalog");
    assert_eq!(descriptor.mcp_tool_name(), "compute_path_inventory");
    let contract = pale_core::canon::parse_strict(descriptor.canonical_contract_bytes())
        .expect("descriptor contract is strict canonical JSON");
    assert_eq!(contract["id"], PATH_INVENTORY_ID);
    let payload_schema =
        pale_core::canon::parse_strict(descriptor.canonical_payload_schema_bytes())
            .expect("descriptor payload schema is strict canonical JSON");
    assert_eq!(payload_schema["type"], "object");

    let execution = api
        .invoke_native_action_payload(NativeActionPayloadRequest::new(
            PATH_INVENTORY_ID.to_string(),
            br#"{"style":"posix_relative","paths":["src/./lib.rs"]}"#.to_vec(),
        ))
        .expect("payload-only admission binds the catalog-owned action tag");
    let value = pale_core::canon::parse_strict(execution.canonical_bytes())
        .expect("payload execution remains strict canonical JSON");
    assert_eq!(value["request"]["input"]["kind"], "path_inventory");

    let error = api
        .invoke_native_action_payload(NativeActionPayloadRequest::new(
            PATH_INVENTORY_ID.to_string(),
            b"[]".to_vec(),
        ))
        .expect_err("payload-only admission accepts objects only");
    assert_eq!(error.diagnostic().code(), "malformed_input");
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
}

#[test]
fn repository_action_catalog_admission_owns_context_order_and_registry_parity() {
    let api = api();
    let (catalog, descriptors) = repository_catalog_fixture(&api);
    let profile_digest = catalog.profile_digest.to_string();
    let instance_digest = catalog.instance_digest.to_string();
    let bytes =
        canonical(&serde_json::to_value(&catalog).expect("catalog fixture projects to JSON"));

    let admitted = api
        .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
            bytes.clone(),
            profile_digest.clone(),
            instance_digest.clone(),
            "pale".to_string(),
        ))
        .expect("matching repository catalog is admitted");
    assert_eq!(admitted.profile_digest(), profile_digest);
    assert_eq!(admitted.instance_digest(), instance_digest);
    assert_eq!(admitted.repository_id(), "pale");
    assert_eq!(admitted.descriptors(), descriptors);

    let context_error = api
        .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
            bytes.clone(),
            "c".repeat(64),
            instance_digest.clone(),
            "pale".to_string(),
        ))
        .expect_err("a different profile cannot inherit the catalog");
    assert_eq!(
        context_error.diagnostic().code(),
        "repository_action_context_mismatch"
    );
    assert_policy(
        context_error.diagnostic(),
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::Blocked,
        DiagnosticRetry::AfterStateChange,
    );

    for (profile, instance) in [
        ("not-a-digest".to_string(), instance_digest.clone()),
        (profile_digest.clone(), "not-a-digest".to_string()),
    ] {
        let malformed = api
            .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
                bytes.clone(),
                profile,
                instance,
                "pale".to_string(),
            ))
            .expect_err("malformed expected digests fail before context comparison");
        assert_eq!(
            malformed.diagnostic().code(),
            "invalid_repository_action_context"
        );
        assert_policy(
            malformed.diagnostic(),
            DiagnosticStage::InputAdmission,
            DiagnosticDisposition::InvalidInput,
            DiagnosticRetry::AfterCorrection,
        );
    }

    let malformed_repository = api
        .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
            bytes.clone(),
            profile_digest.clone(),
            instance_digest.clone(),
            "Pale".to_string(),
        ))
        .expect_err("repository expectations use the catalog owner's lexical grammar");
    assert_eq!(
        malformed_repository.diagnostic().code(),
        "invalid_repository_action_context"
    );
    assert_policy(
        malformed_repository.diagnostic(),
        DiagnosticStage::InputAdmission,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );

    let mut reversed = catalog.clone();
    reversed.contracts.reverse();
    let reversed = pale_core::canon::to_canonical_string(&reversed)
        .expect("reversed test catalog remains canonical JSON")
        .into_bytes();
    let order_error = api
        .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
            reversed,
            profile_digest.clone(),
            instance_digest.clone(),
            "pale".to_string(),
        ))
        .expect_err("contract order is part of semantic admission");
    assert_eq!(
        order_error.diagnostic().code(),
        "repository_action_catalog_unstable"
    );
    assert_policy(
        order_error.diagnostic(),
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );

    let mut unknown = catalog.clone();
    unknown.contracts.truncate(1);
    unknown.contracts[0].id = "pale.compute.unknown".to_string();
    let unknown = pale_core::canon::to_canonical_string(&unknown)
        .expect("unknown test contract remains schema-shaped canonical JSON")
        .into_bytes();
    let unknown_error = api
        .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
            unknown,
            profile_digest.clone(),
            instance_digest.clone(),
            "pale".to_string(),
        ))
        .expect_err("schema-valid unknown actions remain unavailable");
    assert_eq!(
        unknown_error.diagnostic().code(),
        "repository_action_unavailable"
    );
    assert_eq!(
        unknown_error.diagnostic().subject(),
        Some(("action", "pale.compute.unknown"))
    );
    assert_policy(
        unknown_error.diagnostic(),
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::Blocked,
        DiagnosticRetry::AfterStateChange,
    );

    let mut duplicate = catalog.clone();
    duplicate.contracts = vec![duplicate.contracts[0].clone(); 2];
    let duplicate = pale_core::canon::to_canonical_string(&duplicate)
        .expect("duplicate test catalog remains canonical JSON")
        .into_bytes();
    let duplicate_error = api
        .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
            duplicate,
            profile_digest.clone(),
            instance_digest.clone(),
            "pale".to_string(),
        ))
        .expect_err("duplicate contracts fail generated structural admission");
    assert_eq!(duplicate_error.diagnostic().code(), "contract_rejected");
    assert_policy(
        duplicate_error.diagnostic(),
        DiagnosticStage::ContractValidation,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );

    let mut mismatched = catalog;
    mismatched.contracts[0].description.push_str(" changed");
    let mismatched = pale_core::canon::to_canonical_string(&mismatched)
        .expect("mismatched test catalog remains canonical JSON")
        .into_bytes();
    let parity_error = api
        .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
            mismatched,
            profile_digest,
            instance_digest,
            "pale".to_string(),
        ))
        .expect_err("repository contracts cannot drift from the runtime registry");
    assert_eq!(
        parity_error.diagnostic().code(),
        "repository_action_contract_mismatch"
    );
    assert_policy(
        parity_error.diagnostic(),
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::Blocked,
        DiagnosticRetry::AfterStateChange,
    );
}

#[test]
fn malformed_action_style_preserves_one_safe_located_diagnostic_for_both_ingresses() {
    let api = api();
    let payload = br#"{"style":"PRIVATE-REJECTED-VALUE","paths":["src/./lib.rs"]}"#;
    let full = serde_json::json!({
        "kind": "path_inventory",
        "payload": serde_json::from_slice::<serde_json::Value>(payload).unwrap(),
    });
    let complete = api
        .invoke_native_action(NativeActionRequest::new(
            PATH_INVENTORY_ID.to_owned(),
            serde_json::to_vec(&full).unwrap(),
        ))
        .unwrap_err();
    let payload = api
        .invoke_native_action_payload(NativeActionPayloadRequest::new(
            PATH_INVENTORY_ID.to_owned(),
            payload.to_vec(),
        ))
        .unwrap_err();
    assert_eq!(
        complete.diagnostic().canonical_bytes(),
        payload.diagnostic().canonical_bytes()
    );
    let diagnostic: serde_json::Value =
        serde_json::from_slice(complete.diagnostic().canonical_bytes()).unwrap();
    assert_eq!(diagnostic["code"], "malformed_input");
    assert_eq!(diagnostic["stage"], "native_action");
    assert_eq!(diagnostic["disposition"], "invalid_input");
    assert_eq!(diagnostic["retry"], "after_correction");
    assert_eq!(diagnostic["effect"], "not_applicable");
    assert_eq!(diagnostic["source"]["value_path"], "/payload/style");
    assert_eq!(
        diagnostic["schema_coordinate"]["registry_id"],
        "action-input.v1"
    );
    assert_eq!(
        diagnostic["schema_coordinate"]["schema_sha256"],
        pale_contracts::schema_descriptor("action-input.v1")
            .expect("action input descriptor is embedded")
            .schema_sha256
    );
    assert_eq!(
        diagnostic["schema_coordinate"]["schema_path"],
        "/$defs/PathStyle/oneOf"
    );
    let schema_error = complete
        .source()
        .and_then(std::error::Error::source)
        .and_then(|source| source.downcast_ref::<pale_contracts::SchemaError>())
        .expect("typed schema evidence survives public projection");
    let pale_contracts::SchemaError::Invalid(violations) = schema_error else {
        panic!("the rejected style must retain schema violations");
    };
    assert_eq!(
        diagnostic["facts"],
        serde_json::json!([
            {"key": "schema_keyword", "value": "oneOf"},
            {"key": "schema_expectation", "value": violations.violations()[0].expectation()},
            {"key": "retained_violations", "value": "1"},
            {"key": "omitted_violations", "value": "0"},
        ])
    );
    assert!(complete.diagnostic().causes().is_empty());
    let summary = diagnostic["summary"].as_str().unwrap();
    assert_eq!(summary, schema_error.to_string());
    assert!(summary.contains("/payload/style"));
    assert!(summary.contains("posix_relative"));
    assert!(summary.contains("posix_absolute"));
    assert!(
        !String::from_utf8_lossy(complete.diagnostic().canonical_bytes())
            .contains("PRIVATE-REJECTED-VALUE")
    );
    assert!(!format!("{complete:?} {complete}").contains("PRIVATE-REJECTED-VALUE"));
}

#[test]
fn logical_repository_id_runtime_owner_matches_the_generated_contract() {
    let api = api();
    let (catalog, _) = repository_catalog_fixture(&api);
    let cases = [
        ("".to_string(), false),
        ("a".to_string(), true),
        ("a".repeat(255), true),
        ("a".repeat(256), false),
        ("1repo".to_string(), false),
        ("-repo".to_string(), false),
        ("a-".to_string(), true),
        ("a0".to_string(), true),
        ("a-b".to_string(), true),
        ("Pale".to_string(), false),
        ("a_b".to_string(), true),
        ("a/b".to_string(), true),
        ("a.b".to_string(), true),
        ("a:b".to_string(), true),
        ("a/".to_string(), false),
        ("aé".to_string(), false),
        ("a\n".to_string(), false),
    ];

    for (repository_id, expected) in cases {
        let runtime =
            pale_core::records::LogicalRepositoryIdV1::parse(repository_id.clone()).is_ok();
        let mut value = serde_json::to_value(&catalog).expect("catalog fixture serializes");
        value["repository_id"] = serde_json::Value::String(repository_id.clone());
        let generated =
            pale_contracts::validate_value(&value, "repository-native-action-catalog.v1").is_ok();

        assert_eq!(
            runtime, expected,
            "runtime grammar drifted for {repository_id:?}"
        );
        assert_eq!(
            generated, expected,
            "generated grammar drifted for {repository_id:?}"
        );
    }
}
