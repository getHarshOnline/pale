//! Laws for the closed MCP projection.

use super::*;

#[path = "configuration_review.rs"]
mod configuration_review;
#[path = "profile_explanation.rs"]
mod profile_explanation;
#[path = "../support/review.rs"]
mod review_fixture;

#[test]
fn projected_capabilities_equal_the_api_owned_mcp_allowlist() {
    verify_projected_capabilities().expect("MCP projection matches API eligibility");
    assert_eq!(
        projected_capability_ids(),
        capability_inventory()
            .iter()
            .filter(|capability| capability.eligible_for(ApiLeaf::Mcp))
            .map(|capability| capability.id())
            .collect()
    );
}

#[test]
fn mcp_actions_preserve_payload_and_repository_admission_boundaries() {
    let projected = projected_capability_ids();
    for capability in [
        ApiCapabilityId::NativeActionCatalog,
        ApiCapabilityId::NativeActionPayloadInvocation,
    ] {
        assert!(
            projected.contains(&capability),
            "MCP must project {capability:?} through its registry-derived action tools"
        );
    }
    for capability in [
        ApiCapabilityId::NativeActionInvocation,
        ApiCapabilityId::RepositoryNativeActionAdmission,
    ] {
        assert!(
            !projected.contains(&capability),
            "MCP must not project the {capability:?} boundary"
        );
    }
}

#[test]
fn native_action_tools_are_derived_from_every_dispatcher_descriptor() {
    let dispatcher = pale_cli::command::CommandDispatcher::new().unwrap();
    let catalog = dispatcher
        .dispatch(pale_cli::command::CommandRequest::NativeActionCatalog)
        .unwrap();
    let pale_cli::command::CommandOutcome::NativeActionCatalog(descriptors) = catalog else {
        panic!("native-action catalog returned an incompatible outcome");
    };

    for descriptor in descriptors {
        let registration = tools::native_action_registration(&descriptor).unwrap();
        assert_eq!(
            registration.descriptor.name.as_ref(),
            descriptor.mcp_tool_name()
        );
        assert_eq!(
            registration.descriptor.description.as_deref(),
            Some(descriptor.description())
        );
        assert_eq!(
            registration.descriptor.input_schema.as_ref(),
            &serde_json::from_slice::<rmcp::model::JsonObject>(
                descriptor.canonical_payload_schema_bytes()
            )
            .unwrap()
        );
        assert!(registration.descriptor.title.is_some());
        let expected = rmcp::handler::server::tool::schema_for_output::<
            pale_cli::command::NativeActionExecution,
        >();
        assert_eq!(
            registration.descriptor.output_schema.as_deref(),
            Some(expected.as_ref()),
            "native actions share the exact catalog-owned execution contract",
        );
        assert!(matches!(
            registration.route,
            tools::ToolRoute::NativeAction { ref action_id }
                if action_id == descriptor.action_id()
        ));
    }
}

#[test]
fn tool_inventory_is_lexically_unique_and_declares_effect_boundaries() {
    let registrations = tools::FacadeTool::registrations(false, false).unwrap();
    let names = registrations
        .iter()
        .map(|registration| registration.descriptor.name.as_ref())
        .collect::<Vec<_>>();
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(names.contains(&"inspect_effective_profile"));
    assert!(names.contains(&"explain_effective_profile_entry"));
    assert!(names.contains(&"review_configuration"));
    assert!(
        !names.contains(&"observe_resident_operation"),
        "the MCP inventory must not advertise unavailable resident composition"
    );

    let attached = tools::FacadeTool::registrations(true, true).unwrap();
    let titles = attached
        .iter()
        .map(|registration| {
            registration
                .descriptor
                .title
                .as_deref()
                .expect("every attached MCP tool has a concise title")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        titles
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        titles.len(),
        "attached MCP tool titles are unique"
    );
    let observation = attached
        .iter()
        .find(|registration| registration.descriptor.name == "observe_resident_operation")
        .expect("an attached dispatcher advertises resident observation");
    assert_eq!(
        observation.descriptor.title.as_deref(),
        Some("Observe resident operation")
    );
    let expected_observation_schema = rmcp::handler::server::tool::schema_for_output::<
        pale_cli::command::ResidentOperationObservation,
    >();
    assert_eq!(
        observation.descriptor.output_schema.as_deref(),
        Some(expected_observation_schema.as_ref())
    );
    let observation_schema = serde_json::to_value(
        observation
            .descriptor
            .output_schema
            .as_deref()
            .expect("resident Observe advertises its SDK-owned output schema"),
    )
    .expect("resident Observe output schema serializes");
    let observation_properties = observation_schema["properties"]
        .as_object()
        .expect("resident Observe output is an object schema");
    for field in [
        "operationHandleCanonicalJson",
        "operationObservationCanonicalJson",
    ] {
        assert_eq!(observation_properties[field]["type"], "string");
        assert!(observation_properties[field]["description"]
            .as_str()
            .is_some_and(|description| description.contains("canonical JSON text")));
    }
    assert!(!observation_properties.contains_key("operationHandleCanonicalBytes"));
    assert!(!observation_properties.contains_key("operationObservationCanonicalBytes"));
    assert_eq!(observation_schema["additionalProperties"], false);

    for (name, title) in [
        ("start_resident_invoke", "Start resident Invoke"),
        ("advance_resident_invoke", "Advance resident Invoke"),
        ("start_resident_consult", "Start resident Consult"),
        ("advance_resident_consult", "Advance resident Consult"),
        ("start_resident_delegate", "Start resident Delegate"),
        ("advance_resident_delegate", "Advance resident Delegate"),
        ("cancel_resident_operation", "Cancel resident operation"),
    ] {
        let registration = attached
            .iter()
            .find(|registration| registration.descriptor.name == name)
            .unwrap_or_else(|| panic!("attached MCP inventory contains {name}"));
        assert_eq!(registration.descriptor.title.as_deref(), Some(title));
        let input_schema = serde_json::to_value(registration.descriptor.input_schema.as_ref())
            .expect("resident input schema serializes");
        for property in input_schema["properties"]
            .as_object()
            .expect("resident input is an object schema")
            .values()
        {
            assert!(property["description"]
                .as_str()
                .is_some_and(|description| !description.is_empty()));
        }
    }

    for (name, description) in [
        (
            "advance_resident_invoke",
            "Exact pre-signed Invoke continuation request as canonical JSON text.",
        ),
        (
            "advance_resident_consult",
            "Exact pre-signed Consult continuation request as canonical JSON text.",
        ),
        (
            "advance_resident_delegate",
            "Exact pre-signed Delegate continuation request as canonical JSON text.",
        ),
        (
            "cancel_resident_operation",
            "Exact pre-signed Cancel request as canonical JSON text.",
        ),
    ] {
        let registration = attached
            .iter()
            .find(|registration| registration.descriptor.name == name)
            .unwrap_or_else(|| panic!("attached MCP inventory contains {name}"));
        let input_schema = serde_json::to_value(registration.descriptor.input_schema.as_ref())
            .expect("resident continuation input schema serializes");
        assert_eq!(
            input_schema["properties"]["signedCommonRequest"]["description"], description,
            "{name} describes its own signed request"
        );
    }

    let cancellation = attached
        .iter()
        .find(|registration| registration.descriptor.name == "cancel_resident_operation")
        .expect("an attached dispatcher advertises resident cancellation");
    let cancellation_schema = serde_json::to_string(
        cancellation
            .descriptor
            .output_schema
            .as_deref()
            .expect("resident cancellation advertises its SDK-owned output schema"),
    )
    .expect("the cancellation output schema serializes");
    assert!(cancellation_schema.contains("\"terminal_already_established\""));
    assert!(!cancellation_schema.contains("\"established\""));

    let command = tools::FacadeTool::ObserveResidentOperation
        .command(
            serde_json::json!({
                "operationHandleRecordRef": "operation-handle/test",
                "operationHandleSha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "signedCommonRequest": "{}",
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .unwrap();
    let pale_cli::command::CommandRequest::ResidentOperationObservation(request) = command else {
        panic!("resident tool selected the wrong command")
    };
    assert_eq!(
        request.locator().operation_handle_record_ref(),
        "operation-handle/test"
    );
    assert_eq!(request.signed_common_request(), b"{}");

    for registration in registrations {
        let title = registration
            .descriptor
            .title
            .as_deref()
            .expect("every MCP tool has a concise title");
        assert!(!title.is_empty());
        assert!(!title.contains('\n'));
        assert_ne!(
            registration.descriptor.description.as_deref(),
            Some(title),
            "title and description serve different discovery roles"
        );
        let annotations = registration
            .descriptor
            .annotations
            .expect("every MCP tool declares effect hints");
        let (read_only, destructive, idempotent, open_world) = match registration.route {
            tools::ToolRoute::Facade(
                tools::FacadeTool::StartResidentInvoke
                | tools::FacadeTool::StartResidentConsult
                | tools::FacadeTool::StartResidentDelegate,
            ) => (false, false, true, false),
            tools::ToolRoute::Facade(
                tools::FacadeTool::AdvanceResidentInvoke
                | tools::FacadeTool::AdvanceResidentConsult
                | tools::FacadeTool::AdvanceResidentDelegate,
            ) => (false, true, false, true),
            tools::ToolRoute::Facade(tools::FacadeTool::CancelResidentOperation) => {
                (false, true, true, true)
            }
            _ => (true, false, true, false),
        };
        assert_eq!(annotations.read_only_hint, Some(read_only));
        assert_eq!(annotations.destructive_hint, Some(destructive));
        assert_eq!(annotations.idempotent_hint, Some(idempotent));
        assert_eq!(annotations.open_world_hint, Some(open_world));
    }
}

fn invalid_arguments(
    route: tools::FacadeTool,
    arguments: serde_json::Value,
) -> rmcp::model::ErrorData {
    route
        .command(arguments.as_object().expect("fixture is an object").clone())
        .expect_err("malformed tool arguments must be rejected")
}

fn argument_error_data(error: &rmcp::model::ErrorData) -> &serde_json::Value {
    assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_PARAMS);
    assert_eq!(
        error.message,
        "tool arguments do not satisfy the advertised Pale input shape"
    );
    error
        .data
        .as_ref()
        .expect("invalid arguments carry safe detail")
}

#[test]
fn invalid_tool_arguments_report_the_nested_field_without_the_rejected_value() {
    let secret = "do-not-reflect-this-source-body";
    let error = invalid_arguments(
        tools::FacadeTool::CompilePortableWorkspace,
        serde_json::json!({
            "workspace": {
                "rootDocument": "pale.toml",
                "documents": [{"label": "pale.toml", "content": {"secret": secret}}]
            }
        }),
    );
    let data = argument_error_data(&error);

    assert_eq!(data["decoder_class"], "invalid_type");
    assert_eq!(data["path"], "$.workspace.documents[0].content");
    assert_eq!(data["path_truncated"], false);
    assert!(!data.to_string().contains(secret));
}

#[test]
fn invalid_tool_arguments_distinguish_missing_and_unknown_fields() {
    let missing = invalid_arguments(
        tools::FacadeTool::ValidateContract,
        serde_json::json!({"schemaId": "action-input.v1"}),
    );
    let missing_data = argument_error_data(&missing);
    assert_eq!(missing_data["decoder_class"], "missing_field");
    assert_eq!(missing_data["path"], "$.document");

    let secret_key = "secret-that-must-not-be-reflected";
    let unknown = invalid_arguments(
        tools::FacadeTool::CompilePortableWorkspace,
        serde_json::json!({
            "workspace": {
                "rootDocument": "pale.toml",
                "documents": [],
                (secret_key): true,
            }
        }),
    );
    let unknown_data = argument_error_data(&unknown);
    assert_eq!(unknown_data["decoder_class"], "unknown_field");
    assert_eq!(unknown_data["path"], "$.workspace");
    assert!(!unknown_data.to_string().contains(secret_key));
}

#[test]
fn invalid_tool_argument_detail_is_bounded_and_valid_arguments_are_unchanged() {
    let secret_key = "x".repeat(4_096);
    let malformed = invalid_arguments(
        tools::FacadeTool::ValidateContract,
        serde_json::json!({
            "schemaId": "action-input.v1",
            "document": "{}",
            (secret_key.clone()): true,
        }),
    );
    let encoded = serde_json::to_vec(argument_error_data(&malformed)).unwrap();
    assert!(encoded.len() <= 256, "error detail stays transport-bounded");
    assert!(!String::from_utf8(encoded).unwrap().contains(&secret_key));

    let valid = tools::FacadeTool::ValidateContract
        .command(
            serde_json::json!({
                "schemaId": "action-input.v1",
                "document": "{}",
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .expect("valid arguments retain the existing dispatcher route");
    assert!(matches!(
        valid,
        pale_cli::command::CommandRequest::ValidateContractDocument(_)
    ));
}

#[test]
fn output_schemas_are_advertised_only_for_exact_sdk_owned_shapes() {
    let registrations = tools::FacadeTool::registrations(false, false).unwrap();
    for registration in registrations {
        let has_schema = registration.descriptor.output_schema.is_some();
        assert_eq!(
            has_schema,
            matches!(
                registration.route,
                tools::ToolRoute::Facade(
                    tools::FacadeTool::CompilePortableWorkspace
                        | tools::FacadeTool::ComparePortableWorkspaces
                        | tools::FacadeTool::ListPortableWorkspaceChangeRoots
                        | tools::FacadeTool::ListPortableWorkspaceImpacts
                        | tools::FacadeTool::ExplainPortableWorkspaceSubject
                        | tools::FacadeTool::InspectEffectiveProfile
                        | tools::FacadeTool::ExplainEffectiveProfileEntry
                        | tools::FacadeTool::ReviewConfiguration
                        | tools::FacadeTool::CompileOneShot
                        | tools::FacadeTool::ListOneShotObservationTargets
                        | tools::FacadeTool::PlanOneShot
                )
            ),
            "{} output-schema support must remain exact",
            registration.descriptor.name,
        );
    }
}

#[test]
fn protocol_prefers_stateless_revision_and_supports_initialized_clients() {
    assert_eq!(
        protocol::protocol_version(),
        rmcp::model::ProtocolVersion::V_2026_07_28
    );
    assert_eq!(
        protocol::supported_protocol_versions().as_ref(),
        &[
            rmcp::model::ProtocolVersion::V_2026_07_28,
            rmcp::model::ProtocolVersion::V_2025_11_25,
            rmcp::model::ProtocolVersion::V_2025_06_18,
        ]
    );
}

#[test]
fn effective_profile_tool_projects_the_api_owned_canonical_object() {
    let arguments = serde_json::json!({
        "compilation": review_fixture::baseline(),
        "section": "capabilities-and-operations",
        "offset": 0,
        "limit": 7,
    });
    let route = tools::FacadeTool::InspectEffectiveProfile;
    let command = route
        .command(
            arguments
                .as_object()
                .expect("arguments are an object")
                .clone(),
        )
        .expect("MCP wire input admits");
    let outcome = pale_cli::command::CommandDispatcher::new()
        .unwrap()
        .dispatch(command)
        .expect("shared dispatcher compiles the profile");
    let pale_cli::command::CommandOutcome::EffectiveProfilePage(page) = &outcome else {
        panic!("dispatcher returned the wrong closed outcome");
    };
    let expected: serde_json::Value = serde_json::from_slice(page.canonical_bytes())
        .expect("API-owned page bytes are strict JSON");
    let projected = route.project(outcome).expect("MCP projection succeeds");

    assert_eq!(projected.structured_content, Some(expected));
}

#[test]
fn workspace_query_tools_preserve_the_sdk_canonical_objects() {
    let dispatcher = pale_cli::command::CommandDispatcher::new().unwrap();
    let review = review_fixture::preview();
    let prior = review["baseline"]["workspace"].clone();
    let current = review["candidate"]["workspace"].clone();

    let project = |route: tools::FacadeTool, arguments: serde_json::Value| {
        let command = route
            .command(
                arguments
                    .as_object()
                    .expect("arguments are an object")
                    .clone(),
            )
            .expect("MCP wire input admits");
        let outcome = dispatcher
            .dispatch(command)
            .expect("shared dispatcher answers the workspace query");
        let expected = match &outcome {
            pale_cli::command::CommandOutcome::PortableWorkspaceChangeRoots(page) => {
                page.canonical_bytes()
            }
            pale_cli::command::CommandOutcome::PortableWorkspaceImpacts(page) => {
                page.canonical_bytes()
            }
            pale_cli::command::CommandOutcome::PortableWorkspaceExplanation(explanation) => {
                explanation.canonical_bytes()
            }
            _ => panic!("workspace query returned the wrong closed outcome"),
        };
        let expected: serde_json::Value =
            serde_json::from_slice(expected).expect("SDK-owned bytes are strict JSON");
        let projected = route.project(outcome).expect("MCP projection succeeds");
        assert_eq!(projected.structured_content, Some(expected.clone()));
        expected
    };

    project(
        tools::FacadeTool::ListPortableWorkspaceChangeRoots,
        serde_json::json!({
            "prior": prior,
            "current": current,
            "offset": 0,
            "limit": 64,
        }),
    );
    let impacts = project(
        tools::FacadeTool::ListPortableWorkspaceImpacts,
        serde_json::json!({
            "prior": review["baseline"]["workspace"],
            "current": review["candidate"]["workspace"],
            "offset": 0,
            "limit": 64,
        }),
    );
    let subject = impacts["entries"]
        .as_array()
        .and_then(|entries| entries.first())
        .expect("comparison fixture has an impact subject")["subject"]
        .clone();
    project(
        tools::FacadeTool::ExplainPortableWorkspaceSubject,
        serde_json::json!({
            "prior": review["baseline"]["workspace"],
            "current": review["candidate"]["workspace"],
            "subject": subject,
        }),
    );
}
