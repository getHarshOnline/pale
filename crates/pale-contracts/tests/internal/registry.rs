//! Runtime-registry identity, compilation, and generated-bundle tests.

use std::path::{Path, PathBuf};

use super::{
    collect_branch_violations, collect_schema_violations, compile_validator, embedded_schema,
    schema_registry_identity, schema_semantic_discriminator, validate_definition, validate_value,
    verify_generated_source_identity, CachedSchemaError, CompiledSchema, SchemaError,
    SchemaInstancePathSegment, SchemaViolation, EMBEDDED_SCHEMAS, MAX_SCHEMA_VIOLATIONS,
};
use pale_core::records::{LifecycleEvent, RecordSchemaIdentity, SchemaDigest};
use pale_core::sha256_hex;

// Exact negative witness retained without keeping predecessor schema bytes or
// exposing a fallback registry. The active descriptor is the sole authority.
const PREDECESSOR_SESSION_ADMISSION_LEASE_SCHEMA_SHA256: &str =
    "9a04951f2c2f822fbe8d9882e9ebe1e64b5359f462b81584b067a48300d6bee8";
const PREDECESSOR_COMMON_REQUEST_ENVELOPE_SCHEMA_SHA256: &str =
    "a2f5dd1acffe988efa2f2df9fea42ad645200f4dc86f730c2b40727d8467c11f";

fn bundle_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/bundle/agent-surface.v1")
}

#[test]
fn resource_path_schema_and_rust_accept_the_same_identities() {
    use pale_core::records::{PackageIdV1, PackageResourceIdV1, MAX_PACKAGE_ID_BYTES};

    let maximum = format!("a/{}", "b".repeat(MAX_PACKAGE_ID_BYTES - 2));
    let oversized = format!("{maximum}b");
    for candidate in [
        "motion",
        "motion/time-models",
        "web/css",
        "tools/v1.check",
        "",
        "/motion",
        "motion/",
        "motion//time",
        "../motion",
        "motion/../time",
        "motion/./time",
        "motion\\time",
        "motion/%2e%2e",
        "motion/time#heading",
        "motion/time?mode=1",
        "motion/Time",
        "motion/ time",
        "motion/1time",
        "motion/éclair",
        &maximum,
        &oversized,
    ] {
        let value = serde_json::json!(candidate);
        assert_eq!(
            validate_definition(&value, "capability-procedure.v1", "PackageResourceIdV1").is_ok(),
            PackageResourceIdV1::parse(candidate).is_ok(),
            "resource identity: {candidate}"
        );
        assert_eq!(
            validate_definition(&value, "capability-procedure.v1", "PackageIdV1").is_ok(),
            PackageIdV1::parse(candidate).is_ok(),
            "Package identity: {candidate}"
        );
    }
}

#[test]
fn qualified_root_branch_retains_field_evidence_without_rejected_values() {
    let schema = serde_json::json!({"oneOf": [
        {"type": "object", "properties": {"kind": {"const": "other"}}},
        {"type": "object", "properties": {
            "kind": {"const": "selected"},
            "payload": {"properties": {"style": {"oneOf": [
                {"const": "a\"b", "type": "string"},
                {"const": "c\\d", "type": "string"}
            ]}}}
        }}
    ]});
    let validator = compile_validator(&schema).unwrap();
    let instance = serde_json::json!({
        "kind": "selected", "payload": {"style": "DO-NOT-RETAIN"}
    });
    let refined = collect_branch_violations(
        "synthetic.v1",
        &instance,
        validator.iter_errors(&instance),
        Some(1),
    )
    .unwrap();
    let failure = &refined.violations()[0];
    assert_eq!(failure.instance_path(), "/payload/style");
    assert_eq!(
        failure.instance_path_segments(),
        Some(
            [
                SchemaInstancePathSegment::Property("payload".into()),
                SchemaInstancePathSegment::Property("style".into()),
            ]
            .as_slice()
        )
    );
    assert_eq!(
        failure.expectation(),
        r#"one of the schema-declared strings: "a\"b", "c\\d""#
    );
    assert!(!format!("{refined:?} {refined}").contains("DO-NOT-RETAIN"));
    for branch in [None, Some(99)] {
        let aggregate = collect_branch_violations(
            "synthetic.v1",
            &instance,
            validator.iter_errors(&instance),
            branch,
        )
        .unwrap();
        assert_eq!(aggregate.violations()[0].instance_path(), "");
        assert_eq!(aggregate.violations()[0].keyword(), "oneOf");
    }
}

#[test]
fn diagnostic_branch_selection_never_changes_admission_or_other_unions() {
    for schema in [
        serde_json::json!({"oneOf": [{"type":"object"}, {"type":"object"}]}),
        serde_json::json!({"properties": {"nested": {"oneOf": [
            {"type":"integer"}, {"type":"boolean"}
        ]}}}),
    ] {
        let validator = compile_validator(&schema).unwrap();
        let instance = serde_json::json!({"nested":"private"});
        let aggregate =
            collect_schema_violations("synthetic.v1", &instance, validator.iter_errors(&instance));
        let selected = collect_branch_violations(
            "synthetic.v1",
            &instance,
            validator.iter_errors(&instance),
            Some(0),
        );
        assert_eq!(aggregate, selected);
    }
    for value in [
        serde_json::json!({"kind":"path_inventory", "payload":{
            "style":"posix_relative", "paths":[]
        }}),
        serde_json::json!({"kind":"unknown"}),
        serde_json::Value::Null,
    ] {
        for index in [None, Some(0), Some(999)] {
            assert_eq!(
                validate_value(&value, "action-input.v1").is_ok(),
                super::validate_value_with_root_branch(&value, "action-input.v1", index).is_ok(),
            );
        }
    }
}

#[test]
fn reported_string_requirements_do_not_claim_complete_branch_validity() {
    let schema = serde_json::json!({"oneOf": [{"properties": {
        "value": {"oneOf": [
            {"const": "aa", "minLength": 3},
            {"const": "bb", "minLength": 3}
        ]}
    }}]});
    let validator = compile_validator(&schema).unwrap();
    let instance = serde_json::json!({"value": "zzz"});
    let failure = collect_branch_violations(
        "synthetic.v1",
        &instance,
        validator.iter_errors(&instance),
        Some(0),
    )
    .unwrap();
    assert_eq!(
        failure.violations()[0].expectation(),
        "one of the schema-declared strings: \"aa\", \"bb\""
    );
    for value in ["aa", "bb"] {
        assert!(
            !validator.is_valid(&serde_json::json!({"value": value})),
            "a constant requirement does not erase the branch's other constraints"
        );
    }
}

#[test]
fn selected_branch_retention_keeps_the_existing_bound_and_ordering() {
    let properties = (0..(MAX_SCHEMA_VIOLATIONS + 9))
        .map(|index| {
            (
                format!("field-{index:03}"),
                serde_json::json!({"type":"integer"}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let schema = serde_json::json!({"oneOf": [{"properties": properties}]});
    let validator = compile_validator(&schema).unwrap();
    let instance = serde_json::Value::Object(
        properties
            .keys()
            .map(|name| (name.clone(), serde_json::json!("private")))
            .collect(),
    );
    let violations = collect_branch_violations(
        "synthetic.v1",
        &instance,
        validator.iter_errors(&instance),
        Some(0),
    )
    .unwrap();
    assert_eq!(violations.violations().len(), MAX_SCHEMA_VIOLATIONS);
    assert_eq!(violations.truncated_count(), 9);
    assert!(violations
        .violations()
        .windows(2)
        .all(|pair| pair[0] <= pair[1]));
    assert!(violations
        .violations()
        .iter()
        .all(|item| item.keyword() == "type"));
}

#[test]
fn semantic_violation_identity_excludes_expectation_presentation() {
    let first = SchemaViolation {
        instance_path: "/operation".into(),
        instance_path_segments: Some(vec![SchemaInstancePathSegment::Property(
            "operation".into(),
        )]),
        schema_path: "/properties/operation/type".into(),
        keyword: "type".into(),
        instance_path_identity: pale_core::sha256_hex(b"/operation"),
        schema_path_identity: pale_core::sha256_hex(b"/properties/operation/type"),
        keyword_identity: pale_core::sha256_hex(b"type"),
        semantic_discriminator: String::new(),
        expectation: "first renderer wording".into(),
    };
    let second = SchemaViolation {
        expectation: "different renderer wording".into(),
        ..first.clone()
    };

    assert_eq!(first.semantic_identity(), second.semantic_identity());
    assert_ne!(first.to_string(), second.to_string());
}

#[test]
fn required_property_identity_distinguishes_rules_at_the_same_schema_path() {
    let violations_for = |property: &str| {
        let schema = serde_json::json!({
            "type": "object",
            "required": [property],
        });
        let validator = jsonschema::validator_for(&schema).expect("compile synthetic schema");
        let instance = serde_json::json!({});
        collect_schema_violations("synthetic.v1", &instance, validator.iter_errors(&instance))
            .expect("the required property is absent")
    };

    let alpha = violations_for("alpha");
    let beta = violations_for("beta");
    assert_ne!(
        alpha.violations()[0].semantic_identity(),
        beta.violations()[0].semantic_identity(),
        "one required keyword can emit distinct schema-owned obligations"
    );
}

#[test]
fn required_property_retention_is_permutation_invariant_above_the_bound() {
    let properties = (0..(MAX_SCHEMA_VIOLATIONS + 9))
        .map(|index| format!("property-{index:03}"))
        .collect::<Vec<_>>();
    let retained = |required: Vec<String>| {
        let schema = serde_json::json!({
            "type": "object",
            "required": required,
        });
        let validator = jsonschema::validator_for(&schema).expect("compile synthetic schema");
        let instance = serde_json::json!({});
        collect_schema_violations("synthetic.v1", &instance, validator.iter_errors(&instance))
            .expect("all required properties are absent")
            .violations()
            .iter()
            .map(SchemaViolation::semantic_identity)
            .map(|identity| {
                (
                    identity.0.to_string(),
                    identity.1.to_string(),
                    identity.2.to_string(),
                    identity.3.to_string(),
                )
            })
            .collect::<Vec<_>>()
    };

    let forward = retained(properties.clone());
    let mut reversed = properties;
    reversed.reverse();
    assert_eq!(forward, retained(reversed));
    assert_eq!(forward.len(), MAX_SCHEMA_VIOLATIONS);
}

#[test]
fn validator_kind_aliases_have_distinct_semantic_discriminators() {
    use jsonschema::error::ValidationErrorKind;

    assert_ne!(
        schema_semantic_discriminator(&ValidationErrorKind::Pattern {
            pattern: "^a$".into(),
        }),
        schema_semantic_discriminator(&ValidationErrorKind::RegexEngineFailure {
            message: "presentation-only engine message".into(),
        })
    );
    assert_ne!(
        schema_semantic_discriminator(&ValidationErrorKind::ContentEncoding {
            content_encoding: "base64".into(),
        }),
        schema_semantic_discriminator(&ValidationErrorKind::FromUtf8 {
            error: String::from_utf8(vec![0xff]).expect_err("fixture is invalid UTF-8"),
        })
    );
    assert_ne!(
        schema_semantic_discriminator(&ValidationErrorKind::OneOfMultipleValid {
            context: Vec::new(),
        }),
        schema_semantic_discriminator(&ValidationErrorKind::OneOfNotValid {
            context: Vec::new(),
        })
    );
}

#[test]
fn required_property_discriminator_hashes_the_complete_schema_name() {
    use jsonschema::error::ValidationErrorKind;

    let prefix = "x".repeat(1_024);
    let first = ValidationErrorKind::Required {
        property: serde_json::Value::String(format!("{prefix}-alpha")),
    };
    let second = ValidationErrorKind::Required {
        property: serde_json::Value::String(format!("{prefix}-beta")),
    };
    assert_ne!(
        schema_semantic_discriminator(&first),
        schema_semantic_discriminator(&second)
    );
}

#[test]
fn long_sibling_instance_paths_keep_distinct_semantic_identities() {
    let prefix = "x".repeat(1_100);
    let alpha = format!("{prefix}-alpha");
    let beta = format!("{prefix}-beta");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            (alpha.clone()): { "type": "integer" },
            (beta.clone()): { "type": "integer" },
        },
    });
    let instance = serde_json::json!({
        (alpha): "invalid",
        (beta): "invalid",
    });
    let validator = jsonschema::validator_for(&schema).expect("compile synthetic schema");
    let violations =
        collect_schema_violations("synthetic.v1", &instance, validator.iter_errors(&instance))
            .expect("both long sibling values violate their type rules");

    assert_eq!(violations.violations().len(), 2);
    assert_eq!(
        violations.violations()[0].instance_path(),
        violations.violations()[1].instance_path(),
        "bounded presentation prefixes intentionally coincide"
    );
    assert_ne!(
        violations.violations()[0].semantic_identity(),
        violations.violations()[1].semantic_identity(),
        "retention identity must bind complete coordinates"
    );
}

#[test]
fn schema_instance_paths_preserve_numeric_properties_and_array_indices() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "123": { "type": "integer" },
            "items": { "type": "array", "items": { "type": "integer" } },
        },
    });
    let instance = serde_json::json!({ "123": "invalid", "items": ["invalid"] });
    let validator = jsonschema::validator_for(&schema).expect("compile synthetic schema");
    let violations =
        collect_schema_violations("synthetic.v1", &instance, validator.iter_errors(&instance))
            .expect("reject both invalid values");

    let numeric_property = violations
        .violations()
        .iter()
        .find(|violation| violation.instance_path() == "/123")
        .expect("numeric object property violation");
    assert_eq!(
        numeric_property.instance_path_segments(),
        Some([SchemaInstancePathSegment::Property("123".into())].as_slice())
    );

    let array_member = violations
        .violations()
        .iter()
        .find(|violation| violation.instance_path() == "/items/0")
        .expect("array member violation");
    assert_eq!(
        array_member.instance_path_segments(),
        Some(
            [
                SchemaInstancePathSegment::Property("items".into()),
                SchemaInstancePathSegment::Index(0),
            ]
            .as_slice()
        )
    );
}

#[test]
fn schema_instance_paths_decode_rfc6901_property_escapes_once() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "a/b~c": { "type": "integer" } },
    });
    let instance = serde_json::json!({ "a/b~c": "invalid" });
    let validator = jsonschema::validator_for(&schema).expect("compile synthetic schema");
    let violations =
        collect_schema_violations("synthetic.v1", &instance, validator.iter_errors(&instance))
            .expect("reject invalid escaped property");
    let violation = violations.violations().first().expect("one violation");

    assert_eq!(violation.instance_path(), "/a~1b~0c");
    assert_eq!(
        violation.instance_path_segments(),
        Some([SchemaInstancePathSegment::Property("a/b~c".into())].as_slice())
    );
}

fn load_contract_json(relative: &str) -> serde_json::Value {
    let path = bundle_dir().join(relative);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    });
    pale_core::canon::parse_strict(&bytes).unwrap_or_else(|error| {
        panic!("parse {}: {error}", path.display());
    })
}

fn schema_json(id: &str) -> serde_json::Value {
    serde_json::from_str(
        embedded_schema(id).unwrap_or_else(|error| panic!("no schema {id}: {error}")),
    )
    .unwrap_or_else(|error| panic!("parse {id}: {error}"))
}

#[test]
fn capability_procedure_named_definitions_validate_without_a_parallel_schema() {
    let policy = serde_json::json!({
        "permitted_capabilities": ["repository.read"],
        "forbidden_capabilities": [],
        "permitted_effects": ["read"],
        "forbidden_effects": [],
    });
    validate_definition(&policy, "capability-procedure.v1", "CapabilityPolicy")
        .expect("admit a manifest-owned capability policy definition");

    let mut unknown = policy;
    unknown["provider"] = serde_json::json!("must-not-enter-package-procedure");
    let error = validate_definition(&unknown, "capability-procedure.v1", "CapabilityPolicy")
        .expect_err("reject fields outside the named Package procedure definition");
    let SchemaError::Invalid(violations) = error else {
        panic!("expected definition violations, got {error:?}");
    };
    assert_eq!(
        violations.schema_id(),
        "capability-procedure.v1#/$defs/CapabilityPolicy"
    );
    assert!(violations
        .violations()
        .iter()
        .any(|violation| violation.keyword() == "additionalProperties"));

    assert!(matches!(
        validate_definition(&serde_json::json!({}), "capability-procedure.v1", "Absent"),
        Err(SchemaError::UnknownDefinition { .. })
    ));
}

#[test]
fn invalid_instances_retain_safe_structured_schema_reasons() {
    let private_value = "TOP-SECRET-VALUE";
    let error = validate_value(
        &serde_json::json!({
            "exec": private_value,
            "unexpected_secret": private_value,
        }),
        "surface-executable-binding.v1",
    )
    .unwrap_err();
    let SchemaError::Invalid(violations) = error else {
        panic!("expected structured schema violations, got {error:?}");
    };

    assert_eq!(violations.schema_id(), "surface-executable-binding.v1");
    assert!(violations.violations().len() > 1);
    assert_eq!(violations.truncated_count(), 0);
    assert!(violations
        .violations()
        .windows(2)
        .all(|pair| pair[0] <= pair[1]));
    assert!(violations.violations().iter().all(|violation| {
        !violation.instance_path().contains(private_value)
            && !violation.schema_path().contains(private_value)
            && !violation.keyword().contains(private_value)
            && !violation.expectation().contains(private_value)
    }));
    assert!(!violations.to_string().contains(private_value));
    assert!(violations
        .violations()
        .iter()
        .any(|violation| violation.keyword() == "required"));
    assert!(violations
        .violations()
        .iter()
        .any(|violation| violation.keyword() == "additionalProperties"));
}

#[test]
fn schema_violation_retention_is_bounded_and_order_independent() {
    let schema = serde_json::json!({"type": "array", "items": {"type": "integer"}});
    let validator = jsonschema::validator_for(&schema).expect("compile synthetic schema");
    let value = serde_json::Value::Array(
        (0..(MAX_SCHEMA_VIOLATIONS + 9))
            .map(|index| serde_json::Value::String(format!("secret-{index}")))
            .collect(),
    );
    let violations =
        collect_schema_violations("synthetic.v1", &value, validator.iter_errors(&value))
            .expect("reject every string item");

    assert_eq!(violations.violations().len(), MAX_SCHEMA_VIOLATIONS);
    assert_eq!(violations.truncated_count(), 9);
    assert!(violations
        .violations()
        .windows(2)
        .all(|pair| pair[0] <= pair[1]));
    assert!(violations
        .violations()
        .iter()
        .all(|violation| violation.keyword() == "type"
            && violation.expectation() == "JSON type `integer`"));
    assert!(!violations.to_string().contains("secret-"));
}

#[test]
fn schema_compilation_preserves_the_validator_cause() {
    let source = compile_validator(&serde_json::json!({"type": "not-a-json-type"}))
        .expect_err("reject invalid schema keyword value");
    let error = SchemaError::Compile(source);
    let compilation = std::error::Error::source(&error).expect("typed compilation wrapper");
    let validator = compilation.source().expect("jsonschema validator cause");

    assert!(validator
        .downcast_ref::<jsonschema::ValidationError<'static>>()
        .is_some());
}

fn values_under<'a>(value: &'a serde_json::Value, key: &str, out: &mut Vec<&'a serde_json::Value>) {
    match value {
        serde_json::Value::Object(map) => {
            for (candidate, child) in map {
                if candidate == key {
                    out.push(child);
                }
                values_under(child, key, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                values_under(item, key, out);
            }
        }
        _ => {}
    }
}

fn string_literals(value: &serde_json::Value) -> Option<Vec<String>> {
    let literals = if let Some(values) = value.get("enum").and_then(serde_json::Value::as_array) {
        values
            .iter()
            .map(|value| value.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?
    } else {
        value
            .get("oneOf")?
            .as_array()?
            .iter()
            .map(|variant| variant.get("const")?.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?
    };
    let mut values = literals;
    values.sort();
    Some(values)
}

fn enum_under(schema: &serde_json::Value, key: &str) -> Vec<String> {
    let mut nodes = Vec::new();
    values_under(schema, key, &mut nodes);
    nodes
        .into_iter()
        .find_map(|node| {
            string_literals(node).or_else(|| {
                let reference = node.get("$ref")?.as_str()?.strip_prefix('#')?;
                string_literals(schema.pointer(reference)?)
            })
        })
        .unwrap_or_else(|| panic!("no string-literal schema under `{key}`"))
}

fn reference_under<'a>(schema: &'a serde_json::Value, key: &str) -> &'a str {
    let mut nodes = Vec::new();
    values_under(schema, key, &mut nodes);
    nodes
        .into_iter()
        .find_map(|node| {
            let mut references = Vec::new();
            values_under(node, "$ref", &mut references);
            references.into_iter().find_map(serde_json::Value::as_str)
        })
        .unwrap_or_else(|| panic!("no schema reference under `{key}`"))
}

#[test]
fn generated_registry_reproduces_manifest_source_and_set_identity() {
    let manifest = load_contract_json("agent-surface-schemas.v1.json");
    let schemas = manifest["schemas"].as_object().expect("schemas object");
    assert_eq!(EMBEDDED_SCHEMAS.len(), schemas.len());
    let identity = schema_registry_identity().expect("generated registry integrity");
    assert_eq!(
        identity.schema_manifest_sha256.as_str(),
        manifest["schema_manifest_sha256"].as_str().unwrap()
    );
    assert_eq!(
        identity.schema_set_sha256.as_str(),
        manifest["schema_set_sha256"].as_str().unwrap()
    );
    let mut drift = Vec::new();

    for (id, entry) in schemas {
        let relative = entry["path"].as_str().expect("schema path");
        let expected_digest = entry["schema_sha256"].as_str().expect("schema digest");
        let source = std::fs::read(bundle_dir().join(relative)).expect("schema source");
        let actual_digest = sha256_hex(&source);
        if actual_digest != expected_digest {
            drift.push(format!(
                "{id}: manifest {expected_digest}, source {actual_digest}"
            ));
        }
        match embedded_schema(id) {
            Ok(embedded) if embedded.as_bytes() != source => {
                drift.push(format!("{id}: compiled bytes differ from source"));
            }
            Err(error) => drift.push(format!("{id}: source is not embedded: {error}")),
            Ok(_) => {}
        }
    }
    for schema in EMBEDDED_SCHEMAS.iter() {
        if !schemas.contains_key(schema.descriptor.schema_id) {
            drift.push(format!(
                "{}: embedded schema is not pinned",
                schema.descriptor.schema_id
            ));
        }
    }
    assert!(
        drift.is_empty(),
        "schema authority drift:\n{}",
        drift.join("\n")
    );
}

#[test]
fn session_admission_lease_rejects_its_same_id_predecessor_digest() {
    let current = super::record_schema_identity("session-admission-lease.v1")
        .expect("current session-admission lease identity");
    let predecessor = RecordSchemaIdentity {
        schema: current.schema.clone(),
        schema_sha256: SchemaDigest::parse(PREDECESSOR_SESSION_ADMISSION_LEASE_SCHEMA_SHA256)
            .expect("fixed predecessor digest witness"),
    };

    assert_ne!(current.schema_sha256, predecessor.schema_sha256);
    assert!(matches!(
        super::admit_record_schema_identity("session-admission-lease.v1", &predecessor),
        Err(SchemaError::IdentityMismatch(schema_id))
            if schema_id == "session-admission-lease.v1"
    ));
}

#[test]
fn common_request_envelope_rejects_its_same_id_predecessor_digest() {
    let current = super::record_schema_identity("common-request-envelope.v1")
        .expect("current common-request envelope identity");
    let predecessor = RecordSchemaIdentity {
        schema: current.schema.clone(),
        schema_sha256: SchemaDigest::parse(PREDECESSOR_COMMON_REQUEST_ENVELOPE_SCHEMA_SHA256)
            .expect("fixed predecessor digest witness"),
    };

    assert_ne!(current.schema_sha256, predecessor.schema_sha256);
    assert!(matches!(
        super::admit_record_schema_identity("common-request-envelope.v1", &predecessor),
        Err(SchemaError::IdentityMismatch(schema_id))
            if schema_id == "common-request-envelope.v1"
    ));
}

#[test]
fn self_carried_record_identity_resolves_from_its_exact_wire_id() {
    let by_registry = super::record_schema_identity("operation-handle.v1")
        .expect("registered operation-handle identity");
    let by_wire = super::record_schema_identity_by_wire_id("pale/operation-handle.v1")
        .expect("operation-handle wire identity");

    assert_eq!(by_wire, by_registry);
    assert!(matches!(
        super::record_schema_identity_by_wire_id("pale/not-registered.v1"),
        Err(SchemaError::UnknownId(schema_id)) if schema_id == "pale/not-registered.v1"
    ));
}

#[test]
fn generated_source_identity_fields_fail_closed_on_drift() {
    let schema = EMBEDDED_SCHEMAS
        .iter()
        .find(|schema| schema.descriptor.schema_id == "action-execution.v1")
        .expect("generated action-execution descriptor");
    verify_generated_source_identity(&schema.descriptor, schema.source)
        .expect("generated identities match captured source");

    for (schema_uri, wire_schema_id, expected) in [
        (
            "https://example.invalid/drift.json",
            schema.descriptor.wire_schema_id,
            "generated schema URI differs",
        ),
        (
            schema.descriptor.schema_uri,
            Some("pale/action-execution.drift"),
            "generated wire schema ID differs",
        ),
    ] {
        let mut descriptor = schema.descriptor;
        descriptor.schema_uri = schema_uri;
        descriptor.wire_schema_id = wire_schema_id;
        match verify_generated_source_identity(&descriptor, schema.source) {
            Err(CachedSchemaError::Integrity(reason)) => assert!(reason.contains(expected)),
            other => panic!("expected exact identity drift for {expected:?}, got {other:?}"),
        }
    }

    let malformed_source =
        schema
            .source
            .replacen("pale/action-execution.v1", "Pale/action-execution.v1", 1);
    let mut malformed_descriptor = schema.descriptor;
    malformed_descriptor.wire_schema_id = Some("Pale/action-execution.v1");
    match verify_generated_source_identity(&malformed_descriptor, &malformed_source) {
        Err(CachedSchemaError::Integrity(reason)) => {
            assert!(reason.contains("generated wire schema ID is invalid"));
        }
        other => panic!("expected malformed wire schema rejection, got {other:?}"),
    }
}

#[test]
fn every_embedded_schema_compiles_and_reuses_its_validator() {
    for schema in EMBEDDED_SCHEMAS.iter() {
        CompiledSchema::for_id(schema.descriptor.schema_id)
            .unwrap_or_else(|error| panic!("compile {}: {error}", schema.descriptor.schema_id));
    }

    let first = CompiledSchema::for_id("operation-contract.v1").expect("schema compiles");
    let second = CompiledSchema::for_id("operation-contract.v1").expect("schema is cached");
    assert!(std::ptr::eq(first.inner, second.inner));
}

#[test]
fn nested_private_lifecycle_shapes_share_their_authoritative_wire_types() {
    let capabilities = schema_json("capabilities.v1");
    let hook = schema_json("hook-context.v1");
    let lifecycle_event = schema_json("lifecycle-event.v1");
    let lifecycle_event_uri = lifecycle_event["$id"]
        .as_str()
        .expect("lifecycle event $id");

    assert_eq!(
        enum_under(&capabilities, "child_propagation"),
        enum_under(&hook, "child_propagation")
    );
    assert_eq!(reference_under(&hook, "event"), lifecycle_event_uri);
    assert_eq!(reference_under(&capabilities, "event"), lifecycle_event_uri);
    let mut typed_lifecycle_events = LifecycleEvent::vocabulary()
        .map(|event| event.as_str().to_string())
        .collect::<Vec<_>>();
    typed_lifecycle_events.sort();
    assert_eq!(
        string_literals(&lifecycle_event).expect("lifecycle-event string literals"),
        typed_lifecycle_events
    );
    for key in ["action", "predicate_kind"] {
        assert_eq!(enum_under(&hook, key), enum_under(&capabilities, key));
    }
}

#[test]
fn surface_executable_binding_shape_is_identical_before_and_after_resolution() {
    let surface_executable = schema_json("surface-executable-binding.v1");
    let surface_executable_uri = surface_executable["$id"]
        .as_str()
        .expect("surface executable binding $id");
    let mut required = surface_executable["required"]
        .as_array()
        .expect("required array")
        .iter()
        .map(|value| value.as_str().expect("required field").to_string())
        .collect::<Vec<_>>();
    required.sort();
    assert_eq!(required, vec!["digest", "exec", "protocol", "version"]);
    assert_eq!(
        surface_executable["properties"]["digest"]["$ref"].as_str(),
        Some("https://www.rawthoughts.in/pale/schemas/sha256-digest.v1.json")
    );
    for owner in ["machine-binding.v1", "resolved-instance.v1"] {
        assert_eq!(
            reference_under(&schema_json(owner), "surface_executable"),
            surface_executable_uri
        );
    }
}

#[test]
fn native_action_discriminants_match_registry_and_wire_shapes() {
    fn tagged_kinds(schema: &serde_json::Value) -> Vec<String> {
        let mut kinds = schema["oneOf"]
            .as_array()
            .expect("action variants")
            .iter()
            .map(|variant| {
                variant["properties"]["kind"]["const"]
                    .as_str()
                    .expect("action kind")
                    .to_string()
            })
            .collect::<Vec<_>>();
        kinds.sort();
        kinds
    }

    let schema_kinds = tagged_kinds(&schema_json("action-input.v1"));
    assert_eq!(schema_kinds, tagged_kinds(&schema_json("action-output.v1")));

    let contract = schema_json("action-contract.v1");
    assert_eq!(schema_kinds, enum_under(&contract, "input_kind"));
    assert_eq!(schema_kinds, enum_under(&contract, "output_kind"));

    let contract_uri = contract["$id"].as_str().expect("action contract $id");
    for catalog_id in [
        "native-action-catalog.v1",
        "repository-native-action-catalog.v1",
    ] {
        assert_eq!(
            schema_json(catalog_id)["properties"]["contracts"]["items"]["$ref"]
                .as_str()
                .expect("catalog action-contract reference"),
            contract_uri,
        );
    }
}
