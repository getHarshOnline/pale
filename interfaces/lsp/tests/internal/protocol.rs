//! LSP method advertisement and lossless protocol-projection laws.

use super::*;

#[test]
fn authoring_limit_data_names_units_bounds_and_recovery() {
    assert_eq!(
        serde_json::to_value(AuthoringLimitErrorData::query_queue(64)).unwrap(),
        serde_json::json!({
            "kind": "query-queue",
            "unit": "requests",
            "maximum": "64",
            "observedMinimum": "65",
            "recovery": "Wait for one Pale request to finish, then retry against the current editor generation."
        })
    );
    assert_eq!(
        serde_json::to_value(AuthoringLimitErrorData::direct_references()).unwrap(),
        serde_json::json!({
            "kind": "direct-references",
            "unit": "locations",
            "recovery": "Retry with an LSP partial-result token so Pale can stream bounded reference pages."
        })
    );
    assert_eq!(
        serde_json::to_value(AuthoringLimitErrorData::document_bytes(
            "workspace",
            1_025,
            1_024,
        ))
        .unwrap(),
        serde_json::json!({
            "kind": "document-bytes",
            "unit": "bytes",
            "scope": "workspace",
            "maximum": "1024",
            "observed": "1025",
            "recovery": "Reduce this source document, then retry the editor change."
        })
    );
    assert_eq!(
        serde_json::to_value(AuthoringLimitErrorData::document_count("private", 65, 64)).unwrap(),
        serde_json::json!({
            "kind": "document-count",
            "unit": "documents",
            "scope": "private",
            "maximum": "64",
            "observed": "65",
            "recovery": "Remove or combine sources in this custody class, then restart the Pale language server."
        })
    );
    assert_eq!(
        serde_json::to_value(AuthoringLimitErrorData::reference_pages()).unwrap(),
        serde_json::json!({
            "kind": "reference-pages",
            "unit": "pages",
            "maximum": u64::MAX.to_string(),
            "recovery": "Start a new reference request against the current editor generation."
        })
    );
}

#[cfg(feature = "protocol-codegen")]
#[test]
fn schema_export_covers_the_advertised_private_transport() {
    let export = schema::export();
    assert_eq!(export, schema::export());
    let schemas = export["schemas"].as_object().unwrap();
    assert_eq!(
        schemas.keys().map(String::as_str).collect::<Vec<_>>(),
        ["Requests", "Responses"]
    );
    let requests = schemas["Requests"]["properties"].as_object().unwrap();
    let responses = schemas["Responses"]["properties"].as_object().unwrap();
    let methods = export["methods"].as_object().unwrap();
    let mut expected = IMPLEMENTED_METHODS.to_vec();
    expected.sort_unstable();
    assert_eq!(
        methods.keys().map(String::as_str).collect::<Vec<_>>(),
        expected
    );
    for method in methods.values() {
        assert!(requests.contains_key(method["request"].as_str().unwrap()));
        assert!(responses.contains_key(method["response"].as_str().unwrap()));
    }
    assert_eq!(
        export["notifications"][ANALYSIS_PUBLISHED_NOTIFICATION],
        "AnalysisPublication"
    );
    assert!(requests.contains_key(export["initialization"].as_str().unwrap()));
    assert!(responses.contains_key(export["advertisement"].as_str().unwrap()));
    assert!(responses.contains_key(
        export["notifications"][ANALYSIS_PUBLISHED_NOTIFICATION]
            .as_str()
            .unwrap()
    ));
    assert!(schemars::generate::SchemaSettings::draft2020_12()
        .transforms
        .is_empty());
    for bundle in schemas.values() {
        let properties = bundle["properties"].as_object().unwrap();
        assert_eq!(
            bundle["required"],
            serde_json::json!(properties.keys().collect::<Vec<_>>())
        );
        assert!(!bundle["$defs"].as_object().unwrap().is_empty());
    }
    // Equal request types retain separate method aliases in the same namespace.
    assert_eq!(
        requests["CompilationSummaryRequest"],
        requests["ComparisonSummaryRequest"]
    );
    assert_eq!(requests["ChangeRootsRequest"], requests["ImpactsRequest"]);
}

#[cfg(feature = "protocol-codegen")]
#[test]
fn schema_export_preserves_custom_coordinate_serialization() {
    for counter in [0, 9_007_199_254_740_993, u64::MAX] {
        let editor = EditorCoordinate {
            generation: counter,
            document_revision: counter,
        };
        let mut expected = ExpectedProfileCoordinate::try_from(
            serde_json::from_value::<ExpectedProfileCoordinateWire>(profile_coordinate()).unwrap(),
        )
        .unwrap();
        expected.generation = counter;
        expected.document_revision = counter;
        for value in [
            serde_json::to_value(editor).unwrap(),
            serde_json::to_value(expected).unwrap(),
        ] {
            for field in ["generation", "documentRevision"] {
                assert_eq!(value[field], counter.to_string());
                assert_eq!(
                    parse_decimal_coordinate(value[field].as_str().unwrap().to_owned()).unwrap(),
                    counter
                );
            }
        }
    }
    let export = schema::export();
    for name in ["EditorCoordinateOutput", "ExpectedProfileCoordinateOutput"] {
        for field in ["generation", "documentRevision"] {
            let shape = &export["schemas"]["Responses"]["$defs"][name]["properties"][field];
            assert_eq!(shape["type"], "string");
            assert_eq!(shape["format"], "uint64-decimal");
            assert_eq!(shape["minLength"], 1);
            assert_eq!(shape["maxLength"], 20);
        }
    }
    // Shape generation must not silently strengthen request digest parsing.
    let coordinate = ExpectedProfileCoordinate::try_from(
        serde_json::from_value::<ExpectedProfileCoordinateWire>(profile_coordinate()).unwrap(),
    )
    .unwrap();
    assert_eq!(coordinate.profile_digest, "profile");
    for method in [
        COMPILATION_SUMMARY_METHOD,
        COMPARISON_SUMMARY_METHOD,
        PROFILE_CONTEXTS_METHOD,
    ] {
        assert!(decode_empty(method, serde_json::Value::Null).is_ok());
        assert!(decode_empty(method, serde_json::json!({})).is_ok());
        assert!(decode_empty(method, serde_json::json!({"unknown": true})).is_err());
    }
}

fn profile_coordinate() -> serde_json::Value {
    serde_json::json!({
        "generation": "9",
        "documentRevision": "14",
        "profileDigest": "profile",
        "instanceDigest": "instance",
        "selectedTopologyDigest": "topology",
        "workspaceSemanticIrDigest": "workspace-semantic",
        "workspaceAdmissionEvidenceDigest": "workspace-admission",
        "packageSemanticIrDigest": "semantic",
        "compositionPlanDigest": "composition"
    })
}

#[test]
fn advertisement_names_only_callable_custom_methods() {
    assert_eq!(
        experimental_capabilities(),
        serde_json::json!({
            "paleWorkspace": {
                "protocolId": "getharshonline.pale-lsp.workspace",
                "packageVersion": "0.0.1",
                "inventorySha256": "fe47491f1260454669fb888235c09745ce4a229ee00856e3ac57a398fcd6f6df",
                "wireFixtureSha256": WIRE_FIXTURE_SHA256,
                "methods": [
                    "pale/workspace/compilationSummary",
                    "pale/workspace/comparisonSummary",
                    "pale/workspace/changeRoots",
                    "pale/workspace/impacts",
                    "pale/workspace/explain",
                    "pale/workspace/profileContexts",
                    "pale/workspace/effectiveProfile",
                    "pale/workspace/effectiveProfileExplain",
                    "pale/workspace/configurationImprovement"
                ],
                "notifications": ["pale/workspace/analysisPublished"],
                "cancellation": "result-suppression",
                "documentRoles": [
                    "workspace-manifest",
                    "workspace-module",
                    "workspace-language-primitives",
                    "responsibility-derivation",
                    "organizational-translation",
                    "experience-composition",
                    "workspace-resolution-lock",
                    "workspace-universe-lock",
                    "capabilities",
                    "machine-binding",
                    "distribution-lock",
                    "lifecycle-guidance",
                    "package-manifest",
                    "package-doctrine",
                    "package-operations",
                    "package-roles",
                    "package-stacks",
                    "package-run-modes",
                    "package-policy-family",
                    "package-governing-order",
                    "package-capability-policy",
                    "package-tool-recipe",
                    "package-page",
                    "package-data",
                    "package-text"
                ]
            }
        })
    );
}

#[test]
fn advertised_wire_fixture_is_the_exact_locally_attested_exchange() {
    use sha2::{Digest as _, Sha256};

    let fixture = include_bytes!("../fixtures/cross-leaf.v1/effective-profile-wire.json");
    let expected = format!("{:x}", Sha256::digest(fixture));
    assert_eq!(WIRE_FIXTURE_SHA256, expected);
    assert_eq!(
        experimental_capabilities()["paleWorkspace"]["wireFixtureSha256"],
        expected
    );
}

#[test]
fn effective_profile_coordinates_are_typed_and_canonical_decimal_text() {
    let query = decode_query(
        EFFECTIVE_PROFILE_METHOD,
        serde_json::json!({
            "context": {
                "projectId": "project.alpha",
                "executionWorkspaceId": "workspace.alpha"
            },
            "section": "capabilities-and-operations",
            "offset": 64,
            "limit": 64,
            "expectedCoordinate": {
                "generation": "9",
                "documentRevision": "14",
                "profileDigest": "sha256:profile",
                "instanceDigest": "sha256:instance",
                "selectedTopologyDigest": "sha256:topology",
                "workspaceSemanticIrDigest": "sha256:workspace-semantic",
                "workspaceAdmissionEvidenceDigest": "sha256:workspace-admission",
                "packageSemanticIrDigest": "sha256:semantic",
                "compositionPlanDigest": "sha256:composition"
            }
        }),
    )
    .unwrap()
    .unwrap();
    let WorkspaceQuery::EffectiveProfile(query) = query else {
        panic!("effective-profile method returned the wrong query shape");
    };
    assert_eq!(
        query.section,
        EffectiveProfileSection::CapabilitiesAndOperations
    );
    assert_eq!(query.offset, 64);
    assert_eq!(query.limit, 64);
    let expected = query.expected_coordinate.unwrap();
    assert_eq!(expected.generation, 9);
    assert_eq!(expected.document_revision, 14);
    assert_eq!(expected.profile_digest, "sha256:profile");
    assert_eq!(expected.instance_digest, "sha256:instance");
    assert_eq!(expected.selected_topology_digest, "sha256:topology");
    assert_eq!(
        expected.workspace_semantic_ir_digest,
        "sha256:workspace-semantic"
    );
    assert_eq!(
        expected.workspace_admission_evidence_digest,
        "sha256:workspace-admission"
    );
    assert_eq!(expected.package_semantic_ir_digest, "sha256:semantic");
    assert_eq!(expected.composition_plan_digest, "sha256:composition");

    assert!(matches!(
        decode_query(
            EFFECTIVE_PROFILE_METHOD,
            serde_json::json!({
                "context": {
                    "projectId": "project.alpha",
                    "executionWorkspaceId": "workspace.alpha"
                },
                "section": "effective-configuration",
                "offset": 1,
                "limit": 1,
                "expectedCoordinate": {
                    "generation": "09",
                    "documentRevision": "14",
                    "profileDigest": "sha256:profile",
                    "instanceDigest": "sha256:instance",
                    "selectedTopologyDigest": "sha256:topology",
                    "workspaceSemanticIrDigest": "sha256:workspace-semantic",
                    "workspaceAdmissionEvidenceDigest": "sha256:workspace-admission",
                    "packageSemanticIrDigest": "sha256:semantic",
                    "compositionPlanDigest": "sha256:composition"
                }
            })
        ),
        Err(ProtocolError::EditorCoordinate(value)) if value == "09"
    ));
}

#[test]
fn not_admitted_response_serializes_without_fabricated_profile_evidence() {
    let response = EffectiveProfileResponse {
        state: EffectiveProfileState::NotAdmitted {
            summary: "current input was rejected".to_owned(),
        },
        editor: EditorCoordinate {
            generation: 9,
            document_revision: 14,
        },
        context: ProfileInspectionContext {
            project_id: "project.alpha".to_owned(),
            execution_workspace_id: "workspace.alpha".to_owned(),
        },
        coordinates: None,
        page: None,
        diagnostic_identity: None,
    };
    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value["state"]["kind"], "not-admitted");
    assert_eq!(value["editor"]["generation"], "9");
    assert_eq!(value["editor"]["documentRevision"], "14");
    assert!(value.get("coordinates").is_none());
    assert!(value.get("page").is_none());
}

#[test]
fn effective_profile_explanation_requires_an_opaque_handle_and_exact_coordinate() {
    let handle = "a".repeat(64);
    let query = decode_query(
        EFFECTIVE_PROFILE_EXPLANATION_METHOD,
        serde_json::json!({
            "context": {
                "projectId": "project.alpha",
                "executionWorkspaceId": "workspace.alpha"
            },
            "handle": handle,
            "expectedCoordinate": {
                "generation": "9",
                "documentRevision": "14",
                "profileDigest": "profile",
                "instanceDigest": "instance",
                "selectedTopologyDigest": "topology",
                "workspaceSemanticIrDigest": "workspace-semantic",
                "workspaceAdmissionEvidenceDigest": "workspace-admission",
                "packageSemanticIrDigest": "semantic",
                "compositionPlanDigest": "composition"
            }
        }),
    )
    .unwrap()
    .unwrap();
    let WorkspaceQuery::EffectiveProfileExplanation(query) = query else {
        panic!("profile explanation method returned the wrong query shape");
    };
    assert_eq!(query.handle.as_str(), "a".repeat(64));
    assert_eq!(query.expected_coordinate.generation, 9);

    assert!(decode_query(
        EFFECTIVE_PROFILE_EXPLANATION_METHOD,
        serde_json::json!({
            "context": {
                "projectId": "project.alpha",
                "executionWorkspaceId": "workspace.alpha"
            },
            "handle": "not-a-handle",
            "expectedCoordinate": {
                "generation": "9",
                "documentRevision": "14",
                "profileDigest": "profile",
                "instanceDigest": "instance",
                "selectedTopologyDigest": "topology",
                "workspaceSemanticIrDigest": "workspace-semantic",
                "workspaceAdmissionEvidenceDigest": "workspace-admission",
                "packageSemanticIrDigest": "semantic",
                "compositionPlanDigest": "composition"
            }
        }),
    )
    .is_err());
}

#[test]
fn known_methods_decode_to_closed_queries() {
    assert!(matches!(
        decode_query(COMPILATION_SUMMARY_METHOD, serde_json::Value::Null).unwrap(),
        Some(WorkspaceQuery::CompilationSummary)
    ));
    assert!(matches!(
        decode_query(
            CHANGE_ROOTS_METHOD,
            serde_json::json!({"offset": 2, "limit": 5})
        )
        .unwrap(),
        Some(WorkspaceQuery::ChangeRoots(PageParams {
            offset: 2,
            limit: 5
        }))
    ));
    assert!(matches!(
        decode_query(
            lsp_types::request::HoverRequest::METHOD,
            serde_json::json!({
                "textDocument": {"uri": "file:///workspace/pale-workspace.toml"},
                "position": {"line": 0, "character": 0}
            })
        )
        .unwrap(),
        Some(WorkspaceQuery::Authoring(AuthoringQuery::Hover(_)))
    ));
}

#[test]
fn selector_members_remain_separate_and_api_validated() {
    let query = decode_query(
        EXPLANATION_METHOD,
        serde_json::json!({
            "subject": {
                "kind": "declaration",
                "ownerModuleId": "module.alpha",
                "namespaceId": "namespace.alpha",
                "declarationId": "declaration.alpha"
            }
        }),
    )
    .unwrap()
    .unwrap();
    let WorkspaceQuery::Explanation(subject) = query else {
        panic!("explanation method must retain a typed subject");
    };
    assert_eq!(
        SubjectSelector::from(&subject),
        SubjectSelector::Declaration {
            owner_module_id: "module.alpha".to_owned(),
            namespace_id: "namespace.alpha".to_owned(),
            declaration_id: "declaration.alpha".to_owned(),
        }
    );

    assert!(matches!(
        decode_query(
            EXPLANATION_METHOD,
            serde_json::json!({
                "subject": {
                    "kind": "workspace",
                    "workspaceId": "contains spaces"
                }
            })
        ),
        Err(ProtocolError::Selector(_))
    ));
}

#[test]
fn known_methods_reject_extra_or_malformed_parameters() {
    assert!(matches!(
        decode_query(
            COMPILATION_SUMMARY_METHOD,
            serde_json::json!({"unexpected": true})
        ),
        Err(ProtocolError::Parameters { .. })
    ));
    assert!(matches!(
        decode_query(IMPACTS_METHOD, serde_json::json!({"offset": 0})),
        Err(ProtocolError::Parameters { .. })
    ));
    assert!(matches!(
        decode_query(
            EXPLANATION_METHOD,
            serde_json::json!({
                "subject": {
                    "kind": "source",
                    "sourceId": "source.alpha",
                    "unexpected": true
                }
            })
        ),
        Err(ProtocolError::Parameters { .. })
    ));
}

#[test]
fn configuration_improvement_decodes_one_closed_transition_union() {
    let context = serde_json::json!({
        "projectId": "project.alpha",
        "executionWorkspaceId": "workspace.alpha"
    });
    let begin = decode_query(
        CONFIGURATION_IMPROVEMENT_METHOD,
        serde_json::json!({
            "action": "begin",
            "context": context,
            "expectedProfile": profile_coordinate()
        }),
    )
    .unwrap()
    .unwrap();
    assert!(matches!(
        begin,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Begin {
            expected_profile,
            ..
        }) if expected_profile.selected_topology_digest == "topology"
    ));

    let finish = decode_query(
        CONFIGURATION_IMPROVEMENT_METHOD,
        serde_json::json!({
            "action": "finish",
            "context": {
                "projectId": "project.alpha",
                "executionWorkspaceId": "workspace.alpha"
            },
            "expectedPreview": {
                "baseline": profile_coordinate(),
                "candidateEditor": {
                    "generation": "10",
                    "documentRevision": "15"
                },
                "candidateSelectedTopologyDigest": "candidate-topology",
                "candidateProfileDigest": "candidate-profile",
                "candidateInstanceDigest": "candidate-instance",
                "derivationReceiptDigest": "derivation"
            },
            "disposition": "reject"
        }),
    )
    .unwrap()
    .unwrap();
    assert!(matches!(
        finish,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Finish {
            disposition: ImprovementReviewDisposition::Reject,
            expected_preview,
            ..
        }) if expected_preview.candidate_editor.generation == 10
    ));

    let inspect = decode_query(
        CONFIGURATION_IMPROVEMENT_METHOD,
        serde_json::json!({
            "action": "inspect",
            "context": {
                "projectId": "project.alpha",
                "executionWorkspaceId": "workspace.alpha"
            },
            "expectedPreview": {
                "baseline": profile_coordinate(),
                "candidateEditor": {
                    "generation": "10",
                    "documentRevision": "15"
                },
                "candidateSelectedTopologyDigest": "candidate-topology",
                "candidateProfileDigest": "candidate-profile",
                "candidateInstanceDigest": "candidate-instance",
                "derivationReceiptDigest": "derivation"
            },
            "detail": {
                "kind": "subject",
                "subject": {
                    "kind": "source",
                    "sourceId": "source.alpha"
                },
                "inputLimit": 8,
                "maximumInspected": 16,
                "maximumReturned": 4
            }
        }),
    )
    .unwrap()
    .unwrap();
    assert!(matches!(
        inspect,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Inspect {
            detail,
            expected_preview,
            ..
        }) if expected_preview.candidate_editor.document_revision == 15
            && matches!(*detail, ConfigurationImprovementDetailQuery::Subject {
                input_limit: 8,
                maximum_inspected: 16,
                maximum_returned: 4,
                ..
            })
    ));

    assert!(decode_query(
        CONFIGURATION_IMPROVEMENT_METHOD,
        serde_json::json!({
            "action": "begin",
            "context": {
                "projectId": "project.alpha",
                "executionWorkspaceId": "workspace.alpha"
            },
            "expectedProfile": profile_coordinate(),
            "inventedAuthority": true
        }),
    )
    .is_err());
}
