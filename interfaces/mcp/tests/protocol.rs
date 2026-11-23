//! Process-level checks for the standalone stdio transport.

use std::io::Write as _;
use std::process::{Command, Stdio};

#[path = "support/review.rs"]
mod review_fixture;

#[path = "../../cli/tests/support/authoring_failure.rs"]
mod authoring_fixture;

/// Invoke the dedicated review tool in a fresh process for each request.
fn review(input: serde_json::Value) -> serde_json::Value {
    request(serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": params(serde_json::json!({"name": "review_configuration", "arguments": input})),
    }))
}

#[test]
fn standalone_review_reconstructs_and_finishes_each_disposition_without_session_state() {
    let response = review(review_fixture::preview());
    let preview = &response["result"]["structuredContent"];
    assert_eq!(preview["state"], "preview");
    assert_eq!(preview["summary"], review_fixture::expected()["summary"]);
    assert!(preview.get("presentedSummary").is_none());
    let impacts = preview["impacts"]["entries"]
        .as_array()
        .expect("review carries the dispatcher's bounded impact page");
    let expected = review_fixture::expected();
    for (state, count) in [
        ("affected", "affected_count"),
        ("unaffected", "unaffected_count"),
    ] {
        assert_eq!(
            impacts
                .iter()
                .filter(|entry| entry["state"] == state)
                .count() as u64,
            expected["comparison"][count].as_u64().unwrap(),
        );
    }
    assert_eq!(
        impacts.len() as u64,
        expected["comparison"]["affected_count"].as_u64().unwrap()
            + expected["comparison"]["unaffected_count"].as_u64().unwrap(),
    );
    assert!(preview["impacts"]["nextOffset"].is_null());
    let subject = preview["impacts"]["entries"][0]["subject"].clone();
    let details = review(review_fixture::inspect(
        preview["summary"].clone(),
        subject.clone(),
    ));
    let details = &details["result"]["structuredContent"];
    assert_eq!(details["state"], "details");
    assert_eq!(details["summary"], preview["summary"]);
    assert_eq!(details["explanation"]["subject"], subject);
    assert_eq!(details["source"]["subject"], subject);
    assert!(details["source"]["occurrences"].is_array());
    assert!(details["source"]["inputsComplete"].is_boolean());
    assert!(details["source"]["sourcesComplete"].is_boolean());
    assert_eq!(details["sourcePageState"], "retained_review_required");
    assert!(
        !details["source"]["inputsComplete"].as_bool().unwrap()
            || !details["source"]["sourcesComplete"].as_bool().unwrap(),
        "retained-review status must identify unfinished evidence",
    );
    assert!(details["source"].get("recordAnchor").is_some());
    for disposition in ["accepted", "rejected", "cancelled", "stale"] {
        let response = review(review_fixture::finish(
            preview["summary"].clone(),
            disposition,
        ));
        assert_eq!(
            response["result"]["structuredContent"],
            serde_json::json!({
                "state": "finished", "summary": preview["summary"], "disposition": disposition,
            })
        );
    }
}

#[test]
fn standalone_review_rebuilds_sources_before_checked_finalization() {
    let response = review(review_fixture::preview());
    let summary = response["result"]["structuredContent"]["summary"].clone();
    let mut finish = review_fixture::finish(summary, "accepted");
    let binding = &mut finish["baseline"]["privateInputs"]["machineBinding"]["content"];
    *binding = serde_json::Value::String(
        binding
            .as_str()
            .unwrap()
            .replace("/captured/repository", "/captured/changed-after-preview"),
    );

    let response = review(finish);
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(
        response["result"]["structuredContent"]["diagnostic"]["code"],
        "configuration_improvement_summary_mismatch"
    );
    assert_eq!(
        response["result"]["structuredContent"]["diagnostic"]["effect"],
        "not_applicable"
    );
}

#[test]
fn standalone_review_distinguishes_shape_errors_from_canonical_semantic_diagnostics() {
    let mut injected = review_fixture::preview();
    injected["candidate"]["packageDocuments"] = serde_json::json!([]);
    assert_eq!(review(injected)["error"]["code"], -32602);
    let stringly = review(review_fixture::finish(serde_json::json!("{}"), "accepted"));
    assert_eq!(stringly["error"]["code"], -32602);
    for summary in [
        serde_json::json!({}),
        serde_json::json!({"private":"REJECTED-PRIVATE-MARKER"}),
    ] {
        let response = review(review_fixture::finish(summary, "accepted"));
        assert_eq!(response["result"]["isError"], true);
        assert_eq!(
            response["result"]["structuredContent"]["diagnostic"]["code"],
            "configuration_improvement_summary_mismatch"
        );
        assert_eq!(
            response["result"]["structuredContent"]["diagnostic"]["effect"],
            "not_applicable"
        );
        assert!(!serde_json::to_string(&response)
            .unwrap()
            .contains("REJECTED-PRIVATE-MARKER"));
    }
    let mut input = review_fixture::preview();
    input["candidate"]["projectId"] = serde_json::json!("unselected");
    assert_eq!(
        review(input)["result"]["structuredContent"]["diagnostic"],
        review_fixture::expected()["diagnostics"]["context_mismatch"]
    );
}

#[test]
fn standalone_shape_errors_carry_bounded_redacted_decoder_detail() {
    let call = |name: &str, arguments: serde_json::Value| {
        request(serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": params(serde_json::json!({"name": name, "arguments": arguments})),
        }))
    };
    let secret = "do-not-reflect-this-source-body";
    let nested_response = call(
        "compile_portable_workspace",
        serde_json::json!({
            "workspace": {
                "rootDocument": "pale.toml",
                "documents": [{"label": "pale.toml", "content": {"secret": secret}}]
            }
        }),
    );
    assert_eq!(nested_response["error"]["code"], -32602);
    assert_eq!(
        nested_response["error"]["data"],
        serde_json::json!({
            "code": "pale_mcp_invalid_tool_arguments",
            "decoder_class": "invalid_type",
            "path": "$.workspace.documents[0].content",
            "path_truncated": false,
        })
    );
    assert!(!serde_json::to_string(&nested_response)
        .unwrap()
        .contains(secret));

    let missing_response = call(
        "validate_contract",
        serde_json::json!({"schemaId": "action-input.v1"}),
    );
    assert_eq!(
        missing_response["error"]["data"]["decoder_class"],
        "missing_field"
    );
    assert_eq!(missing_response["error"]["data"]["path"], "$.document");

    let secret_key = "x".repeat(4_096);
    let mut unknown = serde_json::json!({
        "schemaId": "action-input.v1",
        "document": "{}",
    });
    unknown
        .as_object_mut()
        .unwrap()
        .insert(secret_key.clone(), serde_json::Value::Bool(true));
    let unknown_response = call("validate_contract", unknown);
    assert_eq!(
        unknown_response["error"]["data"]["decoder_class"],
        "unknown_field"
    );
    assert_eq!(unknown_response["error"]["data"]["path"], "$");
    let encoded = serde_json::to_vec(&unknown_response["error"]["data"]).unwrap();
    assert!(encoded.len() <= 256);
    assert!(!String::from_utf8(encoded).unwrap().contains(&secret_key));
}

/// Reconstruct every profile independently, including the changed private instance.
#[test]
fn standalone_profile_explanation_fences_changed_private_instances() {
    let call = |name: &str, arguments: serde_json::Value| {
        request(serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": params(serde_json::json!({"name": name, "arguments": arguments})),
        }))
    };
    let inspect = |compilation: serde_json::Value| {
        call(
            "inspect_effective_profile",
            serde_json::json!({
                "compilation": compilation, "section": "machine-realization", "offset": 0, "limit": 1,
            }),
        )
    };
    let explain = |compilation: serde_json::Value, handle: serde_json::Value| {
        call(
            "explain_effective_profile_entry",
            serde_json::json!({"compilation": compilation, "handle": handle}),
        )
    };
    let baseline = review_fixture::baseline();
    let prior = inspect(baseline.clone());
    let page = &prior["result"]["structuredContent"];
    let handle = &page["entries"][0]["explanationHandle"];
    let accepted = explain(baseline.clone(), handle.clone());
    assert_eq!(
        accepted["result"]["structuredContent"]["explanation"]["handle"],
        *handle
    );
    assert_eq!(
        accepted["result"]["structuredContent"]["explanation"]["evidence"],
        serde_json::json!({"kind": "machine-realization", "realization": "unknown"})
    );
    let mut changed = baseline.clone();
    let machine = &mut changed["privateInputs"]["machineBinding"]["content"];
    *machine = serde_json::json!(machine.as_str().unwrap().replace(
        "/captured/repository",
        "/captured/repository-another-instance"
    ));
    let current = inspect(changed.clone());
    let current_page = &current["result"]["structuredContent"];
    // Profile pages contain entries, not compilation coordinates. Obtain the
    // independent identity witness from the existing one-shot summary route.
    let prior_compilation = call("compile_one_shot", baseline.clone());
    let changed_compilation = call("compile_one_shot", changed.clone());
    let prior_summary = &prior_compilation["result"]["structuredContent"];
    let changed_summary = &changed_compilation["result"]["structuredContent"];
    assert!(prior_summary["instance_digest"].is_string());
    assert!(changed_summary["instance_digest"].is_string());
    assert_eq!(
        prior_summary["profile_digest"],
        changed_summary["profile_digest"]
    );
    assert_ne!(
        prior_summary["instance_digest"],
        changed_summary["instance_digest"]
    );
    assert_eq!(
        explain(changed.clone(), handle.clone())["result"]["structuredContent"],
        serde_json::json!({"explanation": null})
    );
    let current_handle = &current_page["entries"][0]["explanationHandle"];
    assert_eq!(
        explain(changed, current_handle.clone())["result"]["structuredContent"]["explanation"]
            ["handle"],
        *current_handle
    );
    assert_eq!(
        explain(baseline.clone(), serde_json::json!("0".repeat(64)))["result"]["structuredContent"],
        serde_json::json!({"explanation": null})
    );
    assert_eq!(
        explain(
            baseline.clone(),
            serde_json::json!("private-invalid-handle")
        )["error"]["code"],
        -32602
    );
    let mut invalid = baseline;
    invalid["projectId"] = serde_json::json!("unselected");
    let failure = inspect(invalid.clone());
    let explanation_failure = explain(invalid, handle.clone());
    assert_eq!(failure["result"]["isError"], true);
    assert_eq!(
        failure["result"]["structuredContent"],
        explanation_failure["result"]["structuredContent"]
    );
}

/// Execute one complete JSON-RPC request against a fresh MCP process.
fn request(message: serde_json::Value) -> serde_json::Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pale-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start standalone pale-mcp");
    let mut stdin = child.stdin.take().expect("MCP stdin is piped");
    writeln!(stdin, "{message}").expect("write one bounded JSON-RPC line");
    drop(stdin);
    let output = child.wait_with_output().expect("wait for MCP process");
    assert!(
        output.status.success(),
        "pale-mcp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "successful MCP call is quiet");
    serde_json::from_slice(&output.stdout).expect("MCP emits one JSON response")
}

/// Attach the stable per-request context required by MCP 2026-07-28.
fn params(body: serde_json::Value) -> serde_json::Value {
    let mut body = body.as_object().cloned().unwrap_or_default();
    body.insert(
        "_meta".to_string(),
        serde_json::json!({
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientCapabilities": {},
        }),
    );
    serde_json::Value::Object(body)
}

/// Require one advertised Draft 2020-12 result contract to admit the real result.
fn assert_output_conforms(tool: &serde_json::Value, result: &serde_json::Value) {
    const MAX_CANONICAL_RESULT_BYTES: usize = 256 * 1024;

    let name = tool["name"].as_str().expect("tool has a name");
    let schema = tool
        .get("outputSchema")
        .unwrap_or_else(|| panic!("{name} advertises an output schema"));
    jsonschema::draft202012::meta::validate(schema)
        .unwrap_or_else(|error| panic!("{name} output schema is not Draft 2020-12: {error}"));
    jsonschema::draft202012::validate(schema, result)
        .unwrap_or_else(|error| panic!("{name} result violates its output schema: {error}"));
    let result_bytes = serde_json::to_vec(result).expect("structured result remains JSON");
    assert!(
        result_bytes.len() <= MAX_CANONICAL_RESULT_BYTES,
        "{name} canonical fixture result grew to {} bytes; the qualification ceiling is {MAX_CANONICAL_RESULT_BYTES}",
        result_bytes.len(),
    );
}

#[test]
fn standalone_rejects_requests_without_modern_metadata() {
    let response = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {},
    }));
    assert_eq!(response["error"]["code"], -32602);
    let message = response["error"]["message"]
        .as_str()
        .expect("missing metadata returns one protocol error");
    assert!(message.contains("protocolVersion"));
    assert!(message.contains("clientCapabilities"));
}

#[test]
fn standalone_rejects_malformed_client_capabilities() {
    let response = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": "not-capabilities",
            },
        },
    }));
    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("clientCapabilities")));
}

#[test]
fn standalone_rejects_a_well_formed_unsupported_revision() {
    let response = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2025-11-25",
                "io.modelcontextprotocol/clientCapabilities": {},
            },
        },
    }));
    assert_eq!(response["error"]["code"], -32022);
    assert_eq!(
        response["error"]["data"]["supported"],
        serde_json::json!(["2026-07-28"])
    );
}

#[test]
fn standalone_discovery_reports_the_exact_stateless_surface() {
    let response = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": params(serde_json::json!({})),
    }));
    let result = &response["result"];
    assert_eq!(result["resultType"], "complete");
    assert_eq!(
        result["supportedVersions"],
        serde_json::json!(["2026-07-28", "2025-11-25", "2025-06-18"])
    );
    assert_eq!(result["cacheScope"], "private");
    let server_info = &result["_meta"]["io.modelcontextprotocol/serverInfo"];
    assert_eq!(server_info["name"], "pale-mcp");
    assert_eq!(server_info["version"], "0.0.1");
    assert!(result["capabilities"]["tools"].is_object());
    assert!(result["capabilities"]["resources"].is_object());
    assert!(result["capabilities"]["tasks"].is_null());
}

#[test]
fn standalone_tool_inventory_matches_the_effect_free_command_surface() {
    let response = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": params(serde_json::json!({})),
    }));
    assert_eq!(response["result"]["resultType"], "complete");
    assert_eq!(response["result"]["ttlMs"], 300_000);
    assert_eq!(response["result"]["cacheScope"], "private");
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tool inventory is an array");
    let names = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(tools.len(), 23);
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(names.contains(&"inspect_effective_profile"));
    assert!(names.contains(&"explain_effective_profile_entry"));
    assert!(names.contains(&"compute_path_inventory"));
    for unavailable in [
        "invoke",
        "consult",
        "delegate",
        "observe_operation",
        "cancel_operation",
    ] {
        assert!(
            !names.contains(&unavailable),
            "MCP must not advertise `{unavailable}` before the Engine owns its operation journey"
        );
    }
    assert!(tools.iter().all(|tool| {
        tool["title"]
            .as_str()
            .is_some_and(|title| !title.is_empty() && !title.contains('\n'))
            && tool["title"] != tool["description"]
    }));
    let titles = tools
        .iter()
        .map(|tool| tool["title"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        titles.len(),
        tools.len(),
        "tool titles must be discriminable"
    );
    let action = tools
        .iter()
        .find(|tool| tool["name"] == "compute_path_inventory")
        .unwrap();
    assert_eq!(action["title"], "Compute path inventory");
    assert!(action["outputSchema"].is_object());
    for name in [
        "compile_portable_workspace",
        "compare_portable_workspaces",
        "list_portable_workspace_change_roots",
        "list_portable_workspace_impacts",
        "explain_portable_workspace_subject",
        "inspect_effective_profile",
        "explain_effective_profile_entry",
        "review_configuration",
        "compile_one_shot",
        "list_one_shot_observation_targets",
        "plan_one_shot",
    ] {
        assert!(
            tools.iter().find(|tool| tool["name"] == name).unwrap()["outputSchema"].is_object(),
            "{name} publishes its exact SDK-owned result schema",
        );
    }
    let schematized = tools
        .iter()
        .filter_map(|tool| {
            tool.get("outputSchema")
                .map(|_| tool["name"].as_str().unwrap())
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        schematized,
        std::collections::BTreeSet::from([
            "compare_portable_workspaces",
            "compile_one_shot",
            "compile_portable_workspace",
            "compute_compliance_trace_grade",
            "compute_config_invariant_scan",
            "compute_cost_aggregate",
            "compute_evidence_selection",
            "compute_loop_status_reduce",
            "compute_path_inventory",
            "compute_review_convergence_reduce",
            "compute_rubric_evaluation",
            "compute_service_definition_normalize",
            "compute_skill_health_aggregate",
            "compute_weighted_ranking",
            "explain_portable_workspace_subject",
            "explain_effective_profile_entry",
            "inspect_effective_profile",
            "list_portable_workspace_change_roots",
            "list_portable_workspace_impacts",
            "list_one_shot_observation_targets",
            "plan_one_shot",
            "review_configuration",
        ])
    );
    let validation = tools
        .iter()
        .find(|tool| tool["name"] == "validate_contract")
        .unwrap();
    assert!(
        validation.get("outputSchema").is_none(),
        "validate_contract remains unschematized because its result shape is selected by schemaId",
    );
    for tool in tools
        .iter()
        .filter(|tool| tool.get("outputSchema").is_some())
    {
        jsonschema::draft202012::meta::validate(&tool["outputSchema"]).unwrap_or_else(|error| {
            panic!(
                "{} output schema is not Draft 2020-12: {error}",
                tool["name"]
            )
        });
    }
    let review = tools
        .iter()
        .find(|tool| tool["name"] == "review_configuration")
        .unwrap();
    assert_eq!(review["title"], "Review configuration");
    let output = &review["outputSchema"];
    let details = output["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|variant| variant["properties"]["state"]["const"] == "details")
        .expect("review output describes the details result");
    assert_eq!(
        details["properties"]["sourcePageState"]["$ref"],
        "#/$defs/ConfigurationImprovementSourcePageState",
    );
    let source_ref = details["properties"]["source"]["$ref"]
        .as_str()
        .expect("review details use the SDK-owned source projection");
    let source_schema = output
        .pointer(source_ref.strip_prefix('#').unwrap())
        .unwrap();
    for field in ["dependencyCopiedStringBytes", "sourceCopiedStringBytes"] {
        assert_eq!(source_schema["properties"][field]["type"], "integer");
        assert!(source_schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|required| required == field));
    }
    let source_page_states = output["$defs"]["ConfigurationImprovementSourcePageState"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| variant["const"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(source_page_states, ["complete", "retained_review_required"]);
    let schema = &review["inputSchema"];
    assert_eq!(schema["type"], "object");
    let variants = schema["oneOf"].as_array().unwrap();
    assert_eq!(
        variants
            .iter()
            .map(|variant| variant["properties"]["action"]["const"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["preview", "inspect", "finish"]
    );
    let summary_ref = variants[2]["properties"]["summary"]["$ref"]
        .as_str()
        .expect("finish summary uses the SDK-owned exact shape");
    let summary_schema = schema
        .pointer(summary_ref.strip_prefix('#').unwrap())
        .unwrap();
    assert_eq!(summary_schema["type"], "object");
    assert_eq!(summary_schema["additionalProperties"], false);
    let expected_summary = review_fixture::expected();
    let expected_summary_fields = expected_summary["summary"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let schema_fields = summary_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let required_fields = summary_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field.as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(schema_fields, expected_summary_fields);
    assert_eq!(required_fields, expected_summary_fields);
    let candidate_ref = variants[0]["properties"]["candidate"]["$ref"]
        .as_str()
        .unwrap();
    let candidate = schema
        .pointer(candidate_ref.strip_prefix('#').unwrap())
        .unwrap();
    assert_eq!(candidate["additionalProperties"], false);
    assert!(candidate["properties"].get("packageDocuments").is_none());
    assert!(tools.iter().all(|tool| {
        tool["annotations"]["readOnlyHint"] == true
            && tool["annotations"]["destructiveHint"] == false
            && tool["annotations"]["openWorldHint"] == false
    }));
}

#[test]
fn standalone_list_results_are_complete_private_cacheable_and_deterministic() {
    for (method, collection) in [
        ("tools/list", "tools"),
        ("resources/list", "resources"),
        ("resources/templates/list", "resourceTemplates"),
    ] {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params(serde_json::json!({})),
        });
        let first = request(message.clone());
        let second = request(message);
        let result = &first["result"];

        assert_eq!(result["resultType"], "complete", "{method}");
        assert_eq!(result["ttlMs"], 300_000, "{method}");
        assert_eq!(result["cacheScope"], "private", "{method}");
        assert!(result[collection].is_array(), "{method}");
        assert_eq!(
            result, &second["result"],
            "{method} changed across fresh stateless processes"
        );
    }
}

#[test]
fn standalone_task_methods_remain_unavailable_without_engine_operations() {
    for (method, body) in [
        ("tasks/get", serde_json::json!({"taskId": "operation-1"})),
        (
            "tasks/update",
            serde_json::json!({"taskId": "operation-1", "inputResponses": {}}),
        ),
        ("tasks/cancel", serde_json::json!({"taskId": "operation-1"})),
    ] {
        let response = request(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params(body),
        }));

        assert_eq!(response["error"]["code"], -32601, "{method}");
        assert!(response.get("result").is_none(), "{method}");
    }
}

#[test]
fn standalone_interface_ecology_is_discoverable_bounded_and_recoverable() {
    const MAX_TOOL_INVENTORY_BYTES: usize = 1024 * 1024;
    const MAX_TOOL_DESCRIPTOR_BYTES: usize = 64 * 1024;
    const MAX_INPUT_SCHEMA_BYTES: usize = 16 * 1024;
    const MAX_OUTPUT_SCHEMA_BYTES: usize = 64 * 1024;

    let listed = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": params(serde_json::json!({})),
    }));
    let tools = listed["result"]["tools"]
        .as_array()
        .expect("tool inventory is an array");
    let find = |name: &str| {
        tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("canonical workflow route `{name}` is not discoverable"))
    };
    let compact_len = |value: &serde_json::Value| {
        serde_json::to_vec(value)
            .expect("advertised metadata remains JSON")
            .len()
    };

    let inventory_bytes = compact_len(&listed["result"]["tools"]);
    assert!(
        inventory_bytes <= MAX_TOOL_INVENTORY_BYTES,
        "tool inventory grew to {inventory_bytes} bytes; the qualification ceiling is {MAX_TOOL_INVENTORY_BYTES}",
    );
    for tool in tools {
        let name = tool["name"].as_str().expect("tool has a name");
        let title = tool["title"].as_str().expect("tool has a title");
        let description = tool["description"]
            .as_str()
            .expect("tool has a description");
        assert!(!title.trim().is_empty(), "{name} has an empty title");
        assert!(
            !description.trim().is_empty(),
            "{name} has an empty description"
        );
        assert!(
            compact_len(tool) <= MAX_TOOL_DESCRIPTOR_BYTES,
            "{name} descriptor exceeds its qualification ceiling"
        );

        let input = &tool["inputSchema"];
        jsonschema::draft202012::meta::validate(input)
            .unwrap_or_else(|error| panic!("{name} input schema is not Draft 2020-12: {error}"));
        assert!(
            compact_len(input) <= MAX_INPUT_SCHEMA_BYTES,
            "{name} input schema exceeds its qualification ceiling"
        );
        if let Some(output) = tool.get("outputSchema") {
            jsonschema::draft202012::meta::validate(output).unwrap_or_else(|error| {
                panic!("{name} output schema is not Draft 2020-12: {error}")
            });
            assert!(
                compact_len(output) <= MAX_OUTPUT_SCHEMA_BYTES,
                "{name} output schema exceeds its qualification ceiling"
            );
        }
    }

    // These are evaluation fixtures, not another tool registry. They record
    // the shortest known route sequence for representative initial-v1 tasks.
    let workflows: [(&str, &[&str]); 5] = [
        ("validate one supplied document", &["validate_contract"]),
        (
            "trace one workspace change",
            &[
                "list_portable_workspace_change_roots",
                "list_portable_workspace_impacts",
                "explain_portable_workspace_subject",
            ],
        ),
        (
            "preview and finish one configuration review",
            &["review_configuration", "review_configuration"],
        ),
        (
            "prepare one bounded materialization plan",
            &[
                "compile_one_shot",
                "list_one_shot_observation_targets",
                "plan_one_shot",
            ],
        ),
        (
            "run one effect-free path computation",
            &["compute_path_inventory"],
        ),
    ];
    let measured_calls = workflows.map(|(task, routes)| (task, routes.len()));
    assert_eq!(
        measured_calls,
        [
            ("validate one supplied document", 1),
            ("trace one workspace change", 3),
            ("preview and finish one configuration review", 2),
            ("prepare one bounded materialization plan", 3),
            ("run one effect-free path computation", 1),
        ],
        "canonical workflow call costs changed"
    );
    for (task, routes) in workflows {
        for route in routes {
            assert!(
                find(route)["name"].is_string(),
                "canonical workflow `{task}` lost route `{route}`"
            );
        }
    }

    for (name, cues) in [
        ("validate_contract", &["validate", "schema"][..]),
        (
            "list_portable_workspace_change_roots",
            &["causes", "change"][..],
        ),
        (
            "list_portable_workspace_impacts",
            &["affected", "facts"][..],
        ),
        (
            "explain_portable_workspace_subject",
            &["explain", "subject"][..],
        ),
        ("review_configuration", &["review", "configuration"][..]),
        ("compile_one_shot", &["compile", "configuration"][..]),
        ("list_one_shot_observation_targets", &["paths", "state"][..]),
        ("plan_one_shot", &["plan", "materialization"][..]),
        ("compute_path_inventory", &["path", "inventory"][..]),
    ] {
        let tool = find(name);
        let searchable = format!(
            "{} {}",
            tool["title"].as_str().unwrap(),
            tool["description"].as_str().unwrap()
        )
        .to_ascii_lowercase();
        for cue in cues {
            assert!(
                searchable.contains(cue),
                "{name} metadata omits the canonical discovery cue `{cue}`"
            );
        }
    }

    for (left, right) in [
        ("compile_portable_workspace", "compare_portable_workspaces"),
        (
            "list_portable_workspace_change_roots",
            "list_portable_workspace_impacts",
        ),
        (
            "inspect_effective_profile",
            "explain_effective_profile_entry",
        ),
        ("compile_one_shot", "plan_one_shot"),
    ] {
        let left = find(left);
        let right = find(right);
        assert_ne!(left["title"], right["title"]);
        assert_ne!(left["description"], right["description"]);
    }
    assert_ne!(
        find("compile_portable_workspace")["inputSchema"]["required"],
        find("compare_portable_workspaces")["inputSchema"]["required"],
        "compile and compare must advertise different required inputs"
    );
    assert_eq!(
        find("list_portable_workspace_change_roots")["inputSchema"],
        find("list_portable_workspace_impacts")["inputSchema"],
        "the two page queries deliberately share input shape, so their metadata must distinguish them"
    );

    let missing = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": params(serde_json::json!({
            "name": "validate_contract",
            "arguments": {"schemaId": "action-input.v1"},
        })),
    }));
    assert_eq!(missing["error"]["code"], -32602);
    assert_eq!(missing["error"]["data"]["decoder_class"], "missing_field");
    assert_eq!(missing["error"]["data"]["path"], "$.document");
    assert!(missing["error"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("advertised Pale input shape")));
    assert!(find("validate_contract")["inputSchema"]["required"]
        .as_array()
        .is_some_and(|required| required.iter().any(|field| field == "document")));
}

#[test]
fn standalone_workspace_query_and_review_schemas_admit_real_results() {
    let listed = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": params(serde_json::json!({})),
    }));
    let tools = listed["result"]["tools"]
        .as_array()
        .expect("tool inventory is an array");
    let find = |name: &str| {
        tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("missing tool {name}"))
    };
    let review_input = review_fixture::preview();
    let prior = review_input["baseline"]["workspace"].clone();
    let current = review_input["candidate"]["workspace"].clone();
    let call = |name: &str, arguments: serde_json::Value| {
        request(serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": params(serde_json::json!({"name": name, "arguments": arguments})),
        }))
    };

    let roots = call(
        "list_portable_workspace_change_roots",
        serde_json::json!({"prior": prior, "current": current, "offset": 0, "limit": 64}),
    );
    assert_output_conforms(
        find("list_portable_workspace_change_roots"),
        &roots["result"]["structuredContent"],
    );

    let prior = review_input["baseline"]["workspace"].clone();
    let current = review_input["candidate"]["workspace"].clone();
    let impacts = call(
        "list_portable_workspace_impacts",
        serde_json::json!({"prior": prior, "current": current, "offset": 0, "limit": 64}),
    );
    let impacts_result = &impacts["result"]["structuredContent"];
    assert_output_conforms(find("list_portable_workspace_impacts"), impacts_result);
    let subject = impacts_result["entries"]
        .as_array()
        .and_then(|entries| entries.first())
        .expect("comparison fixture has at least one subject")["subject"]
        .clone();

    let explained = call(
        "explain_portable_workspace_subject",
        serde_json::json!({
            "prior": review_input["baseline"]["workspace"],
            "current": review_input["candidate"]["workspace"],
            "subject": subject,
        }),
    );
    assert_output_conforms(
        find("explain_portable_workspace_subject"),
        &explained["result"]["structuredContent"],
    );

    let preview = review(review_fixture::preview());
    let preview_result = &preview["result"]["structuredContent"];
    assert_output_conforms(find("review_configuration"), preview_result);
    let details = review(review_fixture::inspect(
        preview_result["summary"].clone(),
        subject,
    ));
    assert_output_conforms(
        find("review_configuration"),
        &details["result"]["structuredContent"],
    );
    let finished = review(review_fixture::finish(
        preview_result["summary"].clone(),
        "accepted",
    ));
    assert_output_conforms(
        find("review_configuration"),
        &finished["result"]["structuredContent"],
    );
}

#[test]
fn standalone_one_shot_owner_schemas_admit_real_dispatcher_results() {
    let listed = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": params(serde_json::json!({})),
    }));
    let tools = listed["result"]["tools"]
        .as_array()
        .expect("tool inventory is an array");
    let find = |name: &str| {
        tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("missing tool {name}"))
    };
    let compilation = review_fixture::baseline();

    let compiled = request(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": params(serde_json::json!({
            "name": "compile_one_shot", "arguments": compilation.clone(),
        })),
    }));
    assert_output_conforms(
        find("compile_one_shot"),
        &compiled["result"]["structuredContent"],
    );

    let target_arguments = serde_json::json!({
        "compilation": compilation.clone(),
        "repositoryId": "engine",
        "kind": "apply",
        "prior": {"state": "unowned"},
    });
    let targets = request(serde_json::json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": params(serde_json::json!({
            "name": "list_one_shot_observation_targets", "arguments": target_arguments,
        })),
    }));
    let target_result = &targets["result"]["structuredContent"];
    assert_output_conforms(find("list_one_shot_observation_targets"), target_result);
    let baseline = target_result["targets"]
        .as_array()
        .expect("target result is the exact typed target list")
        .iter()
        .map(|target| {
            serde_json::json!({
                "relativePath": target,
                "observation": {"state": "absent"},
            })
        })
        .collect::<Vec<_>>();

    let planned = request(serde_json::json!({
        "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": params(serde_json::json!({
            "name": "plan_one_shot",
            "arguments": {
                "compilation": compilation,
                "repositoryId": "engine",
                "kind": "apply",
                "prior": {"state": "unowned"},
                "baseline": baseline,
            },
        })),
    }));
    assert_output_conforms(
        find("plan_one_shot"),
        &planned["result"]["structuredContent"],
    );
    assert_eq!(
        planned["result"]["structuredContent"]["schema"],
        "pale.agent-surface-plan.v1"
    );
}

#[test]
fn standalone_native_action_uses_the_registry_payload_schema_and_dispatcher() {
    use pale_cli::command::{
        CommandDispatcher, CommandOutcome, CommandRequest, NativeActionPayloadRequest,
    };

    let dispatcher = CommandDispatcher::new().expect("construct shared dispatcher");
    let catalog = dispatcher
        .dispatch(CommandRequest::NativeActionCatalog)
        .expect("inspect native-action catalog");
    let CommandOutcome::NativeActionCatalog(descriptors) = catalog else {
        panic!("native-action catalog returns its exact outcome");
    };
    let descriptor = descriptors
        .iter()
        .find(|descriptor| descriptor.mcp_tool_name() == "compute_path_inventory")
        .expect("path inventory has one registry-derived MCP name");
    let expected_schema: serde_json::Value =
        serde_json::from_slice(descriptor.canonical_payload_schema_bytes())
            .expect("descriptor payload schema is strict JSON");

    let listed = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": params(serde_json::json!({})),
    }));
    let tool = listed["result"]["tools"]
        .as_array()
        .expect("tool inventory is an array")
        .iter()
        .find(|tool| tool["name"] == "compute_path_inventory")
        .expect("registry-derived action is advertised");
    assert_eq!(tool["inputSchema"], expected_schema);
    assert!(tool["outputSchema"].is_object());

    let payload = serde_json::json!({
        "style": "posix_relative",
        "paths": ["src/./lib.rs"],
    });
    let response = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": params(serde_json::json!({
            "name": "compute_path_inventory",
            "arguments": payload,
        })),
    }));
    let expected = dispatcher
        .dispatch(CommandRequest::NativeActionPayload(
            NativeActionPayloadRequest::new(
                descriptor.action_id().to_owned(),
                serde_json::to_vec(&payload).expect("serialize the same payload object"),
            ),
        ))
        .expect("shared dispatcher executes the payload")
        .into_native_action()
        .expect("payload invocation returns a native-action outcome");
    let expected: serde_json::Value = serde_json::from_slice(expected.canonical_bytes())
        .expect("API execution bytes are strict JSON");
    assert_eq!(response["result"]["structuredContent"], expected);
    assert_output_conforms(tool, &response["result"]["structuredContent"]);

    let invalid_payload = serde_json::json!({
        "style": "PRIVATE-REJECTED-VALUE",
        "paths": [],
    });
    let rejected = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": params(serde_json::json!({
            "name": "compute_path_inventory",
            "arguments": invalid_payload,
        })),
    }));
    assert_eq!(rejected["result"]["isError"], true);
    assert_eq!(
        rejected["result"]["structuredContent"]["diagnostic"]["schema"],
        "pale/diagnostic.v1"
    );
    let expected = dispatcher
        .dispatch(CommandRequest::NativeActionPayload(
            NativeActionPayloadRequest::new(
                descriptor.action_id().to_owned(),
                serde_json::to_vec(&invalid_payload).unwrap(),
            ),
        ))
        .unwrap_err();
    let expected: serde_json::Value =
        serde_json::from_slice(expected.diagnostic().canonical_bytes()).unwrap();
    assert_eq!(
        rejected["result"]["structuredContent"]["diagnostic"],
        expected
    );
    assert_eq!(expected["source"]["value_path"], "/payload/style");
    let response_text = serde_json::to_string(&rejected).unwrap();
    assert!(response_text.contains("posix_relative"));
    assert!(!response_text.contains("PRIVATE-REJECTED-VALUE"));
}

#[test]
fn standalone_contract_resource_uses_the_typed_dispatcher() {
    let response = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/read",
        "params": params(serde_json::json!({
            "uri": "pale://contracts/bundle",
        })),
    }));
    let result = &response["result"];
    assert_eq!(result["resultType"], "complete");
    assert_eq!(result["cacheScope"], "private");
    let contents = result["contents"]
        .as_array()
        .expect("resource result carries content");
    assert_eq!(contents.len(), 1);
    assert_eq!(contents[0]["uri"], "pale://contracts/bundle");
    assert_eq!(contents[0]["mimeType"], "application/json");
    let bundle: serde_json::Value = serde_json::from_str(
        contents[0]["text"]
            .as_str()
            .expect("bundle resource carries API-owned JSON text"),
    )
    .expect("bundle resource text is strict JSON");
    assert!(bundle["memberCount"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert!(bundle["schemaManifestSha256"].is_string());
    assert!(bundle["schemaSetSha256"].is_string());
}

#[test]
fn standalone_tool_failure_preserves_the_api_diagnostic() {
    let response = request(serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": params(serde_json::json!({
            "name": "validate_contract",
            "arguments": {
                "schemaId": "action-input.v1",
                "document": "not JSON",
            },
        })),
    }));
    let result = &response["result"];
    assert_eq!(result["resultType"], "complete");
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"]["diagnostic"]["schema"],
        "pale/diagnostic.v1"
    );
    assert_eq!(
        result["structuredContent"]["diagnostic"]["code"],
        "invalid_json"
    );
    assert_eq!(
        result["structuredContent"]["diagnostic"]["disposition"],
        "invalid_input"
    );
    assert_eq!(
        result["structuredContent"]["diagnostic"]["effect"],
        "not_applicable"
    );
}

#[test]
fn standalone_authoring_failure_keeps_sdk_evidence_in_structured_content() {
    use pale_cli::command::{
        CommandDispatcher, CommandRequest, ContractDocument, PortableWorkspaceDocument,
        PortableWorkspaceRequest,
    };
    for case in 0..3 {
        let mut documents = authoring_fixture::documents();
        if case == 1 {
            documents[0].1 = documents[0]
                .1
                .replace("schema_version = 1", "schema_version = 2");
        }
        let (name, arguments, command) = if case == 2 {
            (
                "validate_contract",
                serde_json::json!({"schemaId":"action-input.v1", "document":"not JSON"}),
                CommandRequest::ValidateContractDocument(ContractDocument::new(
                    "action-input.v1".to_owned(),
                    b"not JSON".to_vec(),
                )),
            )
        } else {
            (
                "compile_portable_workspace",
                serde_json::json!({"workspace": {
                    "rootDocument":"pale-workspace.toml",
                    "documents":documents.iter().map(|(label, content)| serde_json::json!({"label":label, "content":content})).collect::<Vec<_>>()
                }}),
                CommandRequest::PortableWorkspaceCompilation(PortableWorkspaceRequest::new(
                    "pale-workspace.toml".to_owned(),
                    documents
                        .into_iter()
                        .map(|(label, content)| {
                            PortableWorkspaceDocument::new(label.to_owned(), content.into_bytes())
                        })
                        .collect(),
                )),
            )
        };
        let error = CommandDispatcher::new()
            .unwrap()
            .dispatch(command)
            .unwrap_err();
        assert_eq!(error.authoring_corrections().len(), usize::from(case == 0));
        assert_eq!(error.authoring_location().is_some(), case != 2);
        let expected = serde_json::json!({
            "diagnostic":serde_json::from_slice::<serde_json::Value>(error.diagnostic().canonical_bytes()).unwrap(),
            "diagnosticIdentity":error.diagnostic().identity(),
            "authoringCorrections":error.authoring_corrections(),
            "authoringLocation":error.authoring_location(),
        });
        let response = request(serde_json::json!({
            "jsonrpc":"2.0", "id":1, "method":"tools/call",
            "params":params(serde_json::json!({"name":name, "arguments":arguments})),
        }));
        let result = &response["result"];
        assert_eq!(result["isError"], true);
        assert_eq!(result["resultType"], "complete");
        assert_eq!(result["structuredContent"], expected);
        let text = format!(
            "{}: {}",
            error.diagnostic().code(),
            error.diagnostic().summary()
        );
        assert_eq!(
            result["content"],
            serde_json::json!([{"type":"text", "text":text}])
        );
        assert!(result.get("_meta").is_none());
        if case == 0 {
            assert_eq!(
                expected["authoringCorrections"][0]
                    .as_object()
                    .unwrap()
                    .len(),
                7
            );
            assert_eq!(
                expected["authoringCorrections"][0]["diagnostic_identity"],
                expected["diagnosticIdentity"]
            );
        }
    }
}

#[path = "protocol/lifecycle.rs"]
mod lifecycle;
