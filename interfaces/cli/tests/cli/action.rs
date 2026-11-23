//! Native-action equivalence across bounded standard-input and confined-file sources.

use super::*;

fn pretty_terminal_document(canonical: &[u8]) -> Vec<u8> {
    let value: serde_json::Value = serde_json::from_slice(canonical).unwrap();
    let mut output = serde_json::to_string_pretty(&value).unwrap().into_bytes();
    output.push(b'\n');
    output
}

fn rejected_action(format: Option<&str>) -> std::process::Output {
    let mut command = Command::new(bin());
    if let Some(format) = format {
        command.args(["--error-format", format]);
    }
    command
        .args(["action", "pale.compute.does-not-exist"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"{}").unwrap();
    child.wait_with_output().unwrap()
}

fn native_action_descriptors() -> Vec<pale_sdk::NativeActionDescriptor> {
    pale_sdk::PaleApi::new()
        .unwrap()
        .native_action_descriptors()
        .unwrap()
}

#[test]
fn native_action_help_exposes_discovery_without_changing_direct_invocation() {
    let help = Command::new(bin())
        .args(["action", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    let help = String::from_utf8(help.stdout).unwrap();
    for expected in [
        "effect-free built-in computations",
        "list",
        "describe",
        "[ID]",
    ] {
        assert!(
            help.contains(expected),
            "action help omitted `{expected}`:\n{help}"
        );
    }
}

#[test]
fn missing_action_id_points_to_catalog_discovery_in_both_error_formats() {
    let human = Command::new(bin()).arg("action").output().unwrap();
    assert_eq!(human.status.code(), Some(4));
    assert!(human.stdout.is_empty());
    let human_error = String::from_utf8(human.stderr).unwrap();
    assert!(human_error.contains("action ID is required for invocation"));
    assert!(human_error.contains("pale action list"));

    let json = Command::new(bin())
        .args(["--error-format", "json", "action"])
        .output()
        .unwrap();
    assert_eq!(json.status.code(), Some(4));
    assert!(json.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&json.stderr).unwrap();
    assert_eq!(diagnostic["diagnostic"]["code"], "host_input_rejected");
    assert_eq!(
        diagnostic["diagnostic"]["summary"],
        "action ID is required for invocation"
    );
    assert!(diagnostic["diagnostic"]["causes"][0]
        .as_str()
        .unwrap()
        .contains("pale action list"));
}

#[test]
fn native_action_list_projects_the_canonical_catalog_in_human_and_json_forms() {
    let descriptors = native_action_descriptors();

    let human = Command::new(bin())
        .args(["action", "list"])
        .output()
        .unwrap();
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    let mut expected = String::from("Effect-free built-in computations:\n");
    for descriptor in &descriptors {
        expected.push_str(descriptor.action_id());
        expected.push_str("  ");
        expected.push_str(descriptor.description());
        expected.push('\n');
    }
    assert_eq!(String::from_utf8(human.stdout).unwrap(), expected);

    let json = Command::new(bin())
        .args(["--result-format", "json", "action", "list"])
        .output()
        .unwrap();
    assert!(json.status.success());
    assert!(json.stderr.is_empty());
    let expected_json = serde_json::json!({
        "actions": descriptors
            .iter()
            .map(|descriptor| serde_json::json!({
                "action_id": descriptor.action_id(),
                "description": descriptor.description(),
            }))
            .collect::<Vec<_>>(),
    });
    let expected_envelope = serde_json::json!({
        "completion": "complete",
        "result": expected_json,
    });
    assert_eq!(
        json.stdout,
        pretty_terminal_document(&serde_json::to_vec(&expected_envelope).unwrap()),
        "machine-readable discovery must use the shared result envelope"
    );
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let actions = value["result"]["actions"].as_array().unwrap();
    assert_eq!(actions.len(), descriptors.len());
    for (actual, expected) in actions.iter().zip(&descriptors) {
        assert_eq!(actual["action_id"], expected.action_id());
        assert_eq!(actual["description"], expected.description());
        assert_eq!(actual.as_object().unwrap().len(), 2);
    }

    let repeated = Command::new(bin())
        .args(["--result-format", "json", "action", "list"])
        .output()
        .unwrap();
    assert_eq!(repeated.stdout, json.stdout);
    assert_eq!(repeated.stderr, json.stderr);
}

#[test]
fn native_action_describe_returns_exact_contract_and_cli_input_description() {
    let descriptors = native_action_descriptors();
    for descriptor in &descriptors {
        let contract: serde_json::Value =
            serde_json::from_slice(descriptor.canonical_contract_bytes()).unwrap();
        let payload_schema: serde_json::Value =
            serde_json::from_slice(descriptor.canonical_payload_schema_bytes()).unwrap();
        let cli_input_description = serde_json::json!({
            "schema_id": "action-input.v1",
            "kind": contract["input_kind"],
            "payload_schema": payload_schema,
        });
        let expected = serde_json::json!({
            "action_id": descriptor.action_id(),
            "description": descriptor.description(),
            "contract": contract,
            "cli_input_description": cli_input_description,
        });
        let json = Command::new(bin())
            .args([
                "--result-format",
                "json",
                "action",
                "describe",
                descriptor.action_id(),
            ])
            .output()
            .unwrap();
        assert!(json.status.success());
        assert!(json.stderr.is_empty());
        let expected = serde_json::json!({
            "completion": "complete",
            "result": expected,
        });
        assert_eq!(
            json.stdout,
            pretty_terminal_document(&serde_json::to_vec(&expected).unwrap()),
            "CLI description drifted from SDK descriptor {}",
            descriptor.action_id()
        );
    }

    let descriptor = descriptors
        .into_iter()
        .find(|descriptor| descriptor.action_id() == "pale.compute.path-inventory")
        .unwrap();
    let contract: serde_json::Value =
        serde_json::from_slice(descriptor.canonical_contract_bytes()).unwrap();
    let payload_schema: serde_json::Value =
        serde_json::from_slice(descriptor.canonical_payload_schema_bytes()).unwrap();
    let cli_input_description = serde_json::json!({
        "schema_id": "action-input.v1",
        "kind": contract["input_kind"],
        "payload_schema": payload_schema,
    });

    let human = Command::new(bin())
        .args(["action", "describe", descriptor.action_id()])
        .output()
        .unwrap();
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    assert_eq!(
        String::from_utf8(human.stdout).unwrap(),
        format!(
            "{}\n{}\n\nAction contract:\n{}\n\nHow to construct CLI input:\n{}\n",
            descriptor.action_id(),
            descriptor.description(),
            serde_json::to_string_pretty(&contract).unwrap(),
            serde_json::to_string_pretty(&cli_input_description).unwrap(),
        )
    );

    let json = Command::new(bin())
        .args([
            "--result-format",
            "json",
            "action",
            "describe",
            descriptor.action_id(),
        ])
        .output()
        .unwrap();
    assert!(json.status.success());
    assert!(json.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(value["completion"], "complete");
    let value = &value["result"];
    assert_eq!(value["action_id"], descriptor.action_id());
    assert_eq!(value["description"], descriptor.description());
    assert_eq!(value["contract"], contract);
    assert!(value.get("payload_schema").is_none());
    assert_eq!(value["cli_input_description"], cli_input_description);
    assert_eq!(
        value["cli_input_description"]["schema_id"],
        "action-input.v1"
    );
    assert_eq!(
        value["cli_input_description"]["kind"],
        value["contract"]["input_kind"]
    );
    assert_eq!(
        value["cli_input_description"]["payload_schema"],
        payload_schema
    );
    assert_eq!(payload_schema["type"], "object");
    assert_eq!(payload_schema["additionalProperties"], false);
    assert_eq!(
        payload_schema["required"],
        serde_json::json!(["style", "paths"])
    );
    assert_eq!(payload_schema["properties"]["paths"]["type"], "array");
    assert_eq!(payload_schema["properties"]["paths"]["maxItems"], 100_000);
    assert_eq!(
        payload_schema["properties"]["paths"]["items"]["type"],
        "string"
    );
    assert_eq!(
        payload_schema["properties"]["paths"]["items"]["minLength"],
        1
    );
    assert_eq!(
        payload_schema["properties"]["paths"]["items"]["maxLength"],
        4_096
    );
    assert_eq!(
        payload_schema["properties"]["style"]["$ref"],
        "#/$defs/PathStyle"
    );
}

#[test]
fn native_action_description_guides_construction_of_an_invocable_cli_document() {
    let described = Command::new(bin())
        .args([
            "--result-format",
            "json",
            "action",
            "describe",
            "pale.compute.path-inventory",
        ])
        .output()
        .unwrap();
    assert!(described.status.success());
    let described: serde_json::Value = serde_json::from_slice(&described.stdout).unwrap();
    let input = serde_json::json!({
        "kind": described["result"]["cli_input_description"]["kind"],
        "payload": {
            "style": "posix_relative",
            "paths": ["src/./lib.rs"],
        },
    });

    let mut child = Command::new(bin())
        .args(["action", "pale.compute.path-inventory"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&input).unwrap().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let mut bare_payload = Command::new(bin())
        .args([
            "--error-format",
            "json",
            "action",
            "pale.compute.path-inventory",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    bare_payload
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"style":"posix_relative","paths":["src/./lib.rs"]}"#)
        .unwrap();
    let bare_payload = bare_payload.wait_with_output().unwrap();
    assert_eq!(bare_payload.status.code(), Some(4));
    assert!(bare_payload.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&bare_payload.stderr).unwrap();
    assert_eq!(diagnostic["diagnostic"]["code"], "malformed_input");
}

#[test]
fn native_action_describe_rejects_an_unknown_catalog_identity() {
    let output = Command::new(bin())
        .args([
            "--error-format",
            "json",
            "action",
            "describe",
            "pale.compute.does-not-exist",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    let diagnostic = &diagnostic["diagnostic"];
    assert_eq!(diagnostic["code"], "host_input_rejected");
    assert_eq!(diagnostic["disposition"], "invalid_input");
    assert_eq!(
        diagnostic["summary"],
        "action ID is not in the compiled catalog; use `pale action list` to see available IDs"
    );
}

#[test]
fn native_action_accepts_the_same_typed_input_from_stdin_or_a_confined_file() {
    let input = r#"{"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs"]}}"#;
    let expected = pale_sdk::PaleApi::new()
        .unwrap()
        .invoke_native_action(pale_sdk::NativeActionRequest::new(
            "pale.compute.path-inventory".to_string(),
            input.as_bytes().to_vec(),
        ))
        .unwrap();
    let mut stdin_child = Command::new(bin())
        .args(["action", "pale.compute.path-inventory"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    stdin_child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let stdin = stdin_child.wait_with_output().unwrap();
    assert!(stdin.status.success());
    assert_eq!(
        stdin.stdout,
        pretty_terminal_document(expected.canonical_bytes())
    );

    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("input.json"), input).unwrap();
    let file = Command::new(bin())
        .args([
            "action",
            "pale.compute.path-inventory",
            "--input-root",
            root.path().to_str().unwrap(),
            "--input-file",
            "input.json",
        ])
        .output()
        .unwrap();
    assert!(file.status.success());
    assert_eq!(stdin.stdout, file.stdout);

    let escaped = Command::new(bin())
        .args(["--error-format", "json"])
        .args([
            "action",
            "pale.compute.path-inventory",
            "--input-root",
            root.path().to_str().unwrap(),
            "--input-file",
            "../input.json",
        ])
        .output()
        .unwrap();
    assert_eq!(escaped.status.code(), Some(4));
    assert!(escaped.stdout.is_empty());
    let expected = pale_sdk::PaleApi::new()
        .unwrap()
        .capture_local_native_action_request(pale_sdk::LocalNativeActionInputRequest::new(
            "pale.compute.path-inventory".to_owned(),
            root.path().to_str().unwrap().to_owned(),
            "../input.json".to_owned(),
        ))
        .unwrap_err();
    assert_eq!(
        escaped.stderr,
        pretty_terminal_document(
            pale_cli::command::DiagnosticPresentation::from_api_error(&expected)
                .to_json()
                .unwrap()
                .as_bytes()
        ),
        "the CLI must preserve the local API diagnostic value without reclassification"
    );
}

#[test]
fn native_action_failures_have_human_and_json_diagnostic_parity() {
    let json = rejected_action(Some("json"));
    assert_eq!(json.status.code(), Some(4));
    assert!(json.stdout.is_empty());
    let expected = pale_sdk::PaleApi::new()
        .unwrap()
        .invoke_native_action(pale_sdk::NativeActionRequest::new(
            "pale.compute.does-not-exist".to_string(),
            b"{}".to_vec(),
        ))
        .unwrap_err();
    assert_eq!(
        json.stderr,
        pretty_terminal_document(
            pale_cli::command::DiagnosticPresentation::from_api_error(&expected)
                .to_json()
                .unwrap()
                .as_bytes()
        ),
        "the CLI JSON leaf must render the API diagnostic value without reclassification"
    );
    let value: serde_json::Value = serde_json::from_slice(&json.stderr).unwrap();
    assert_eq!(
        value["diagnosticIdentity"],
        expected.diagnostic().identity().as_str()
    );
    assert_eq!(value["authoringCorrections"], serde_json::json!([]));
    assert!(value["authoringLocation"].is_null());
    let value = &value["diagnostic"];
    assert_eq!(value["schema"], "pale/diagnostic.v1");
    assert_eq!(value["schema_version"], 1);
    assert!(value["schema_sha256"]
        .as_str()
        .is_some_and(|value| value.len() == 64));
    assert_eq!(value["subject"]["kind"], "action");
    assert_eq!(value["subject"]["id"], "pale.compute.does-not-exist");
    assert_eq!(value["code"], "unknown_action");
    assert_eq!(value["stage"], "native_action");
    assert_eq!(value["disposition"], "invalid_input");
    assert_eq!(value["retry"], "after_correction");
    assert_eq!(value["effect"], "not_applicable");
    assert!(value["truncation"].is_object());
    assert!(value["summary"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    for legacy in [
        "action_id",
        "request_digest",
        "safe_context",
        "truncated_count",
    ] {
        assert!(
            value.get(legacy).is_none(),
            "legacy field `{legacy}` escaped"
        );
    }

    let human = rejected_action(None);
    assert_eq!(human.status.code(), Some(4));
    assert!(human.stdout.is_empty());
    let human = String::from_utf8(human.stderr).unwrap();
    for rendered in [
        value["code"].as_str().unwrap(),
        value["summary"].as_str().unwrap(),
        value["subject"]["id"].as_str().unwrap(),
        value["disposition"].as_str().unwrap(),
        value["retry"].as_str().unwrap(),
        value["effect"].as_str().unwrap(),
    ] {
        assert!(
            human.contains(rendered),
            "human diagnostic omitted admitted value `{rendered}`: {human}"
        );
    }
}

#[test]
fn malformed_action_style_keeps_field_evidence_in_json_and_human_output() {
    let input =
        br#"{"kind":"path_inventory","payload":{"style":"PRIVATE-REJECTED-VALUE","paths":[]}}"#;
    let expected = pale_sdk::PaleApi::new()
        .unwrap()
        .invoke_native_action(pale_sdk::NativeActionRequest::new(
            "pale.compute.path-inventory".to_owned(),
            input.to_vec(),
        ))
        .unwrap_err();
    for format in ["json", "human"] {
        let mut child = Command::new(bin())
            .args([
                "--error-format",
                format,
                "action",
                "pale.compute.path-inventory",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(4));
        assert!(output.stdout.is_empty());
        let rendered = std::str::from_utf8(&output.stderr).unwrap();
        assert!(rendered.contains("/payload/style"));
        assert!(rendered.contains("posix_relative"));
        assert!(!rendered.contains("PRIVATE-REJECTED-VALUE"));
        if format == "json" {
            assert_eq!(
                output.stderr,
                pretty_terminal_document(
                    pale_cli::command::DiagnosticPresentation::from_api_error(&expected)
                        .to_json()
                        .unwrap()
                        .as_bytes()
                )
            );
        }
    }
}

#[test]
fn malformed_action_json_is_invalid_input_and_never_contaminates_stdout() {
    let mut child = Command::new(bin())
        .args([
            "--error-format",
            "json",
            "action",
            "pale.compute.path-inventory",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"{broken").unwrap();
    let output = child.wait_with_output().unwrap();

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    let diagnostic = &diagnostic["diagnostic"];
    assert_eq!(diagnostic["schema"], "pale/diagnostic.v1");
    assert_eq!(diagnostic["code"], "invalid_json");
}
