//! Focused conformance tests for the private complete-corpus generator.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use pale_contract_codegen::{generate_bundle, read_descriptor_manifest, verify_bundle};

const BUNDLE: &str = "contracts/bundle/agent-surface.v1";
const MANIFEST: &str = "agent-surface-schemas.v1.json";
#[test]
fn generated_catalog_is_complete_and_unique() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let expected = manifest
        .active_schema_ids()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let bundle = generate_bundle(manifest).expect("generate complete bundle");
    let generated = bundle
        .schemas()
        .map(|(schema_id, _)| schema_id.to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(bundle.schemas().len(), expected.len());
    assert_eq!(
        generated, expected,
        "the generated producer set must equal manifest custody"
    );
}

#[test]
fn executable_identity_contracts_do_not_restore_runtime_identity_names() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let manifest_json = serde_json::to_string(&manifest).expect("serialize descriptor manifest");
    let bundle = generate_bundle(manifest).expect("generate complete bundle");

    let schema_ids = bundle
        .schemas()
        .map(|(schema_id, _)| schema_id)
        .collect::<BTreeSet<_>>();
    assert!(schema_ids.contains("surface-executable-binding.v1"));
    assert!(schema_ids.contains("surface-executable-identity.v1"));

    let mut generated_text = manifest_json;
    for (_, bytes) in bundle.schemas() {
        generated_text.push_str(std::str::from_utf8(bytes).expect("UTF-8 schema"));
    }

    for stale_name in [
        "RuntimeBinding",
        "SurfaceRuntimeIdentityV1",
        "runtime_identity_digest",
        "runtime-binding.v1",
        "surface-runtime-identity.v1",
        "pale.agent-surface-runtime.v1",
        "pale/surface-runtime-identity.v1",
    ] {
        assert!(
            !generated_text.contains(stale_name),
            "stale executable-identity name remains: {stale_name}"
        );
    }

    for schema_id in ["machine-binding.v1", "resolved-instance.v1"] {
        let (_, bytes) = bundle
            .schemas()
            .find(|(candidate, _)| *candidate == schema_id)
            .expect("executable-binding consumer schema");
        let schema: serde_json::Value =
            serde_json::from_slice(bytes).expect("parse executable-binding consumer schema");
        let properties = schema
            .pointer("/properties")
            .and_then(serde_json::Value::as_object)
            .expect("root object properties");
        assert!(
            properties.contains_key("surface_executable"),
            "{schema_id} must name the selected executable explicitly"
        );
        assert!(
            !properties.contains_key("runtime"),
            "{schema_id} restored the ambiguous executable field name"
        );
    }
}

#[test]
fn workspace_id_schema_projects_the_same_maximum_as_rust_admission() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let bundle = generate_bundle(manifest).expect("generate complete bundle");
    let (_, bytes) = bundle
        .schemas()
        .find(|(schema_id, _)| *schema_id == "workspace-module.v1")
        .expect("workspace module schema");
    let schema: serde_json::Value =
        serde_json::from_slice(bytes).expect("parse workspace module schema");
    assert_eq!(
        schema.pointer("/$defs/WorkspaceModuleId/maxLength"),
        Some(&serde_json::json!(255))
    );
}

#[test]
fn signed_trust_contracts_preserve_distinct_key_epoch_domains() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let bundle = generate_bundle(manifest).expect("generate complete bundle");
    let maximum = serde_json::json!(9_007_199_254_740_991_u64);
    for (schema_id, epoch_definition) in [
        ("principal-resident-writer-fence.v1", "PrincipalKeyEpochV1"),
        (
            "principal-resident-writer-terminal-receipt.v1",
            "PrincipalKeyEpochV1",
        ),
        (
            "principal-resident-writer-takeover-receipt.v1",
            "PrincipalKeyEpochV1",
        ),
        ("producer-incarnation.v1", "SigningKeyEpochV1"),
        ("producer-incarnation-retirement.v1", "SigningKeyEpochV1"),
        ("deployment-binding-proposal.v1", "PrincipalKeyEpochV1"),
        ("deployment-acceptance.v1", "SigningKeyEpochV1"),
        ("deployment-acceptance-revocation.v1", "SigningKeyEpochV1"),
        ("deployment-binding.v1", "PrincipalKeyEpochV1"),
        ("deployment-runtime-authority.v1", "PrincipalKeyEpochV1"),
        ("deployment-runtime-authority.v1", "SigningKeyEpochV1"),
    ] {
        let (_, bytes) = bundle
            .schemas()
            .find(|(candidate, _)| *candidate == schema_id)
            .expect("signed trust schema");
        let schema: serde_json::Value =
            serde_json::from_slice(bytes).expect("parse signed trust schema");
        let minimum = format!("/$defs/{epoch_definition}/minimum");
        let maximum_pointer = format!("/$defs/{epoch_definition}/maximum");
        assert_eq!(
            schema.pointer(&minimum),
            Some(&serde_json::json!(1)),
            "{schema_id} must reject key epoch zero"
        );
        assert_eq!(
            schema.pointer(&maximum_pointer),
            Some(&maximum),
            "{schema_id} must reject non-canonical JSON integers"
        );
    }
}

#[test]
fn principal_succession_schemas_project_the_admission_algebra() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let bundle = generate_bundle(manifest).expect("generate complete bundle");
    let schema = |schema_id| {
        let (_, bytes) = bundle
            .schemas()
            .find(|(candidate, _)| *candidate == schema_id)
            .expect("principal succession schema");
        serde_json::from_slice::<serde_json::Value>(bytes)
            .expect("parse principal succession schema")
    };

    for schema_id in [
        "principal-record.v1",
        "principal-recovery-policy.v1",
        "principal-key-transition.v1",
        "principal-recovery-proof.v1",
    ] {
        let schema = schema(schema_id);
        assert!(
            schema
                .pointer("/properties/canonical_record_sha256")
                .is_some(),
            "{schema_id} must expose the common canonical digest member"
        );
        assert!(
            schema
                .pointer("/properties/canonical_principal_record_sha256")
                .is_none(),
            "{schema_id} must not retain the old principal self-digest alias"
        );
    }

    let principal = schema("principal-record.v1");
    assert_eq!(
        principal["oneOf"]
            .as_array()
            .expect("principal transition branches")
            .len(),
        3
    );
    for member in ["key_transition_ref", "key_transition_sha256"] {
        assert_eq!(
            principal["dependentRequired"][member]
                .as_array()
                .expect("principal key-transition dependency")
                .len(),
            1
        );
    }

    let policy = schema("principal-recovery-policy.v1");
    assert!(
        policy
            .pointer("/properties/canonical_recovery_policy_sha256")
            .is_none(),
        "the recovery policy must not retain its old self-digest alias"
    );
    let eligible_signers = policy
        .pointer("/properties/eligible_signers")
        .expect("eligible signer set schema");
    assert_eq!(eligible_signers["minItems"], serde_json::json!(1));
    assert_eq!(eligible_signers["maxItems"], serde_json::json!(1_024));
    assert_eq!(eligible_signers["uniqueItems"], serde_json::json!(true));
    assert_eq!(
        policy.pointer("/properties/threshold/minimum"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        policy["oneOf"]
            .as_array()
            .expect("recovery policy lineage branches")
            .len(),
        2
    );
    for member in ["previous_policy_ref", "previous_policy_sha256"] {
        assert_eq!(
            policy["dependentRequired"][member]
                .as_array()
                .expect("recovery policy predecessor dependency")
                .len(),
            1
        );
    }

    let proof = schema("principal-recovery-proof.v1");
    let signatures = proof
        .pointer("/properties/signatures")
        .expect("recovery signature set schema");
    assert_eq!(signatures["minItems"], serde_json::json!(1));
    assert_eq!(signatures["maxItems"], serde_json::json!(1_024));
    assert_eq!(signatures["uniqueItems"], serde_json::json!(true));

    let transition = schema("principal-key-transition.v1");
    assert_eq!(
        transition["oneOf"]
            .as_array()
            .expect("exclusive transition branches")
            .len(),
        2
    );
    assert_eq!(
        transition
            .pointer("/dependentRequired/recovery_authority_proof_ref")
            .and_then(serde_json::Value::as_array)
            .expect("proof reference digest pair"),
        &[serde_json::json!("recovery_authority_proof_sha256")]
    );
}

#[test]
fn deployment_structural_laws_project_from_the_rust_wire_types() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let bundle = generate_bundle(manifest).expect("generate complete bundle");
    let schema = |schema_id| {
        let (_, bytes) = bundle
            .schemas()
            .find(|(candidate, _)| *candidate == schema_id)
            .expect("deployment schema");
        serde_json::from_slice::<serde_json::Value>(bytes).expect("parse deployment schema")
    };

    for (schema_id, pointer, minimum) in [
        ("contract-set.v1", "/properties/entries", None),
        (
            "deployment-terms.v1",
            "/properties/permitted_purposes",
            None,
        ),
        (
            "deployment-runtime-authority.v1",
            "/properties/permitted_record_schema_ids",
            Some(1),
        ),
    ] {
        let schema = schema(schema_id);
        let collection = schema.pointer(pointer).expect("set-like collection schema");
        assert_eq!(collection["maxItems"], serde_json::json!(1_024));
        assert_eq!(collection["uniqueItems"], serde_json::json!(true));
        if let Some(minimum) = minimum {
            assert_eq!(collection["minItems"], serde_json::json!(minimum));
        }
    }

    let revocation = schema("deployment-acceptance-revocation.v1");
    for member in [
        "previous_revocation_ref",
        "previous_revocation_sha256",
        "previous_revocation_epoch",
    ] {
        assert_eq!(
            revocation["dependentRequired"][member]
                .as_array()
                .expect("revocation predecessor dependency")
                .len(),
            2
        );
    }
    let runtime = schema("deployment-runtime-authority.v1");
    for member in ["previous_authority_ref", "previous_authority_sha256"] {
        assert_eq!(
            runtime["dependentRequired"][member]
                .as_array()
                .expect("runtime-authority predecessor dependency")
                .len(),
            1
        );
    }
}

#[test]
fn capability_result_schemas_project_exact_branch_and_binding_laws() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let bundle = generate_bundle(manifest).expect("generate complete bundle");
    let schema = |schema_id| {
        let (_, bytes) = bundle
            .schemas()
            .find(|(candidate, _)| *candidate == schema_id)
            .expect("capability transport schema");
        serde_json::from_slice::<serde_json::Value>(bytes)
            .expect("parse capability transport schema")
    };

    let result = schema("capability-result.v1");
    assert_eq!(
        result["oneOf"]
            .as_array()
            .expect("terminal result branches")
            .len(),
        2
    );
    assert_eq!(
        result["$defs"]["CapabilitySuccessV1"]["oneOf"]
            .as_array()
            .expect("success representation branches")
            .len(),
        2
    );

    let failure = schema("adapter-failure.v1");
    assert_eq!(
        failure["oneOf"]
            .as_array()
            .expect("failure effect-state branches")
            .len(),
        2
    );
    for member in ["absence_evidence_ref", "absence_evidence_sha256"] {
        assert_eq!(
            failure["dependentRequired"][member]
                .as_array()
                .expect("absence-evidence dependency")
                .len(),
            1
        );
    }
    assert_eq!(
        failure.pointer("/allOf/0/if/properties/failure_class/const"),
        Some(&serde_json::json!("rate_limited"))
    );
    assert_eq!(
        failure.pointer("/allOf/0/then/required"),
        Some(&serde_json::json!(["retry_at"]))
    );
    assert!(result.pointer("/allOf").is_none());

    for schema_id in [
        "adapter-failure.v1",
        "capability-result.v1",
        "effect-dispatch-intent.v1",
        "effect-receipt.v1",
        "reconciliation-result.v1",
    ] {
        let encoded = serde_json::to_string(&schema(schema_id)).expect("schema encodes");
        assert!(
            encoded.contains("CapabilityAttemptIdV1"),
            "{schema_id} must use the common transport-attempt identity"
        );
        assert!(
            !encoded.contains("EffectAttemptIdV1"),
            "{schema_id} must not retain the effect-only predecessor name"
        );
    }

    assert_eq!(
        result.pointer("/$defs/CapabilitySuccessV1/properties/inline_payload/not/type"),
        Some(&serde_json::json!("null"))
    );
    for schema_id in [
        "adapter-failure.v1",
        "capability-result.v1",
        "effect-receipt.v1",
        "reconciliation-result.v1",
    ] {
        let schema = schema(schema_id);
        for pointer in [
            "/$defs/RequestBindingV1/properties/execution_owner_fence_ref",
            "/$defs/RequestBindingV1/properties/execution_owner_fence_sha256",
            "/$defs/RequestBindingV1/properties/federation_request_admission_ref",
            "/$defs/RequestBindingV1/properties/federation_request_admission_sha256",
        ] {
            let property = schema.pointer(pointer).unwrap_or_else(|| {
                panic!("{schema_id} must project optional property `{pointer}`")
            });
            let encoded = serde_json::to_string(property).expect("property schema encodes");
            assert!(
                !encoded.contains(r#""type":"null""#),
                "{schema_id} must represent `{pointer}` absence by omission"
            );
        }
    }
    for schema_id in [
        "adapter-failure.v1",
        "capability-result.v1",
        "common-request-envelope.v1",
    ] {
        let schema = schema(schema_id);
        for pointer in [
            "/$defs/SigningAuthorityBindingV1/properties/integrated_runtime",
            "/$defs/SigningAuthorityBindingV1/properties/integrated_owner",
            "/$defs/SigningAuthorityBindingV1/properties/standalone_owner_native",
        ] {
            let property = schema.pointer(pointer).unwrap_or_else(|| {
                panic!("{schema_id} must project optional property `{pointer}`")
            });
            let encoded = serde_json::to_string(property).expect("property schema encodes");
            assert!(
                !encoded.contains(r#""type":"null""#),
                "{schema_id} must represent `{pointer}` absence by omission"
            );
        }
    }
    let request = schema("common-request-envelope.v1");
    for pointer in [
        "/$defs/CommonRequestRouteV1/properties/destination_node_id",
        "/$defs/CommonRequestScopeV1/properties/relationship_scope",
        "/$defs/CommonRequestSessionAdmissionV1/properties/checked_revocation_ref",
        "/$defs/CommonRequestSessionAdmissionV1/properties/checked_revocation_sha256",
    ] {
        let property = request
            .pointer(pointer)
            .unwrap_or_else(|| panic!("common request must project `{pointer}`"));
        let encoded = serde_json::to_string(property).expect("property schema encodes");
        assert!(
            !encoded.contains(r#""type":"null""#),
            "common request must represent `{pointer}` absence by omission"
        );
    }
    for member in ["checked_revocation_ref", "checked_revocation_sha256"] {
        assert_eq!(
            request["$defs"]["CommonRequestSessionAdmissionV1"]["dependentRequired"][member]
                .as_array()
                .expect("checked-revocation dependency")
                .len(),
            1
        );
    }

    let execution_fence = request["$defs"]["CommonRequestExecutionOwnerFenceV1"]["properties"]
        .as_object()
        .expect("neutral execution-owner fence projection");
    let expected_execution_fence_members = [
        "execution_fence_ref",
        "execution_fence_sha256",
        "execution_attempt_id",
        "execution_path_id",
        "execution_owner_system_id",
        "execution_owner_participant_id",
        "owner_authorization_schema_id",
        "owner_authorization_schema_sha256",
        "owner_authorization_ref",
        "owner_authorization_sha256",
        "owner_generation",
        "control_epoch",
    ];
    for member in expected_execution_fence_members {
        assert!(
            execution_fence.contains_key(member),
            "common request omitted neutral execution-owner member `{member}`"
        );
    }
    let mut required = request["$defs"]["CommonRequestExecutionOwnerFenceV1"]["required"]
        .as_array()
        .expect("required neutral execution-owner coordinates")
        .iter()
        .map(|member| member.as_str().expect("required member name"))
        .collect::<Vec<_>>();
    required.sort_unstable();
    let mut expected_required = expected_execution_fence_members.to_vec();
    expected_required.sort_unstable();
    assert_eq!(required, expected_required);
    for retired in [
        "paperclip_execution_authorization_ref",
        "paperclip_execution_authorization_sha256",
    ] {
        assert!(
            !execution_fence.contains_key(retired),
            "common request retained product-specific member `{retired}`"
        );
    }
}

#[test]
fn generic_responsibility_answers_have_meaningful_schema_names() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let bundle = generate_bundle(manifest).expect("generate complete bundle");
    let (_, bytes) = bundle
        .schemas()
        .find(|(schema_id, _)| *schema_id == "responsibility-derivation.v1")
        .expect("responsibility derivation schema");
    let schema: serde_json::Value =
        serde_json::from_slice(bytes).expect("parse responsibility derivation schema");
    let definitions = schema["$defs"]
        .as_object()
        .expect("responsibility definitions");

    assert!(definitions.contains_key("AttributableAnswerForResponsibilityAdopterModuleV1"));
    assert!(definitions.contains_key("AttributableAnswerForResponsibilityCeilingsV1"));
    assert!(definitions
        .keys()
        .all(|name| !name.starts_with("AttributableAnswerV1") || name == "AttributableAnswerV1"));
    assert_eq!(
        schema.pointer("/properties/adopter_module/$ref"),
        Some(&serde_json::json!(
            "#/$defs/AttributableAnswerForResponsibilityAdopterModuleV1"
        ))
    );
}

#[test]
fn manifest_remains_the_custody_inventory_for_every_producer() {
    let manifest = read_descriptor_manifest(&bundle_dir().join(MANIFEST))
        .expect("admit checked-in descriptor manifest");
    let active_ids = manifest
        .active_schema_ids()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let bundle = generate_bundle(manifest.clone()).expect("generate complete bundle");
    let generated_ids = bundle
        .schemas()
        .map(|(schema_id, _)| schema_id.to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        active_ids, generated_ids,
        "manifest custody and generated resources must be bijective"
    );

    let serialized = serde_json::to_value(&manifest).expect("serialize typed manifest");
    let entries = serialized["schemas"]
        .as_object()
        .expect("typed manifest schemas map");
    assert_eq!(entries.len(), bundle.schemas().len());
    assert_eq!(
        entries.len(),
        138,
        "the initial-v1 registry has 138 producers"
    );
    for obsolete in [
        "connector.v1",
        "lifecycle-behaviour.v1",
        "package-selection.v1",
        "run-policies.v1",
        "tool.v1",
    ] {
        assert!(
            !entries.contains_key(obsolete),
            "obsolete standalone producer `{obsolete}` must stay removed"
        );
    }
    assert!(entries.values().all(|entry| {
        entry.get("record_authority").is_some()
            && entry.pointer("/lifecycle/role").is_some()
            && entry.pointer("/persistence/class").is_some()
    }));
}

#[test]
fn checked_in_bundle_is_the_deterministic_complete_projection() {
    let path = bundle_dir();
    let first = generate_bundle(
        read_descriptor_manifest(&path.join(MANIFEST)).expect("admit descriptor manifest"),
    )
    .expect("generate complete bundle");
    let second = generate_bundle(
        read_descriptor_manifest(&path.join(MANIFEST)).expect("read descriptor manifest again"),
    )
    .expect("regenerate complete bundle");

    assert_eq!(first.coordinate(), second.coordinate());
    assert_eq!(first.manifest_bytes(), second.manifest_bytes());
    assert_eq!(
        first
            .schemas()
            .map(|(id, bytes)| (id.to_owned(), bytes.to_vec()))
            .collect::<Vec<_>>(),
        second
            .schemas()
            .map(|(id, bytes)| (id.to_owned(), bytes.to_vec()))
            .collect::<Vec<_>>()
    );
    verify_bundle(&first, &path).expect("checked-in bundle is byte-current");
}

#[test]
fn bundle_and_abi_identifiers_consume_the_core_candidate_inventory() {
    let source = fs::read(bundle_dir().join(MANIFEST)).expect("read descriptor manifest");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&source).expect("parse descriptor manifest");
    let schemas = manifest["schemas"]
        .as_object_mut()
        .expect("schema inventory");
    let mut entry = schemas
        .remove("responsibility-contract.v1")
        .expect("responsibility descriptor");
    entry["path"] = serde_json::json!("schemas/pale/agent-surface/responsibility-concept.v1.json");
    schemas.insert("responsibility-concept.v1".into(), entry);
    let directory = tempfile::tempdir().expect("temporary manifest directory");
    let path = directory.path().join(MANIFEST);
    fs::write(
        &path,
        serde_json::to_vec(&manifest).expect("serialize mutated manifest"),
    )
    .expect("write mutated manifest");
    let error = read_descriptor_manifest(&path).expect_err("candidate-only schema id is denied");
    assert!(error.to_string().contains("candidate-only token `concept`"));
}

fn bundle_dir() -> PathBuf {
    workspace_root().join(BUNDLE)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate is nested under workspace/crates")
        .to_path_buf()
}
