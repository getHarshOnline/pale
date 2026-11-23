//! API-owner verification for cross-leaf conformance data.

use pale_sdk::{
    AuthoringIngressClass, CompilationDocument, ContractDocument, DiagnosticDisposition,
    DiagnosticEffectCertainty, DiagnosticProjection, DiagnosticRetry, DiagnosticStage,
    EffectiveProfileCompilationRequest, NativeActionPayloadRequest, NativeActionRequest,
    ObservationCancellation, ObservationCategory, ObservationCausalSelector, ObservationFact,
    ObservationFilter, ObservationLensDisposition, ObservationLensLimits, ObservationLensRequest,
    ObservationPageDisposition, ObservationPageLimits, ObservationPageRequest,
    ObservationSubjectKind, OneShotBaselineEntry, OneShotBaselineObservation,
    OneShotCompilationRequest, OneShotPlanKind, OneShotPlanningRequest,
    OneShotPriorMaterialization, OneShotPrivateInputs, PaleApi, PortableWorkspaceDocument,
    PortableWorkspaceExplanationState, PortableWorkspaceImpactState, PortableWorkspaceRequest,
    PortableWorkspaceSubjectKind, RepositoryNativeActionCatalogRequest,
};
#[path = "support/conformance.rs"]
mod conformance;
#[path = "support/repository_catalog.rs"]
mod repository_catalog;

fn fixture(member: &str) -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cross-leaf.v1")
        .join(member);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn captured_private_inputs() -> OneShotPrivateInputs {
    let document =
        |label: &str, bytes: &[u8]| CompilationDocument::new(label.to_owned(), bytes.to_vec());
    OneShotPrivateInputs::new(
        document(
            "capabilities.toml",
            include_bytes!("fixtures/cross-leaf.v1/one-shot/private/capabilities.toml"),
        ),
        document(
            "machines/machine.toml",
            include_bytes!("fixtures/cross-leaf.v1/one-shot/private/machine-binding.toml"),
        ),
        document(
            "distribution-lock.toml",
            include_bytes!("fixtures/cross-leaf.v1/one-shot/private/distribution-lock.toml"),
        ),
        Vec::new(),
    )
}

fn captured_package_documents() -> Vec<CompilationDocument> {
    let document =
        |label: &str, bytes: &[u8]| CompilationDocument::new(label.to_owned(), bytes.to_vec());
    vec![
        document(
            "engine/package.toml",
            include_bytes!("fixtures/cross-leaf.v1/one-shot/packages/engine/package.toml"),
        ),
        document(
            "engine/role.md",
            include_bytes!("fixtures/cross-leaf.v1/one-shot/packages/engine/role.md"),
        ),
        document(
            "engine/roles.toml",
            include_bytes!("fixtures/cross-leaf.v1/one-shot/packages/engine/roles.toml"),
        ),
    ]
}

fn one_shot_request() -> OneShotCompilationRequest {
    OneShotCompilationRequest::new(
        "demo".to_owned(),
        "default".to_owned(),
        "machine".to_owned(),
        captured_private_inputs(),
        captured_package_documents(),
        one_shot_portable_request(),
    )
}

fn effective_profile_request() -> EffectiveProfileCompilationRequest {
    EffectiveProfileCompilationRequest::new(
        "demo".to_owned(),
        "default".to_owned(),
        "machine".to_owned(),
        captured_private_inputs(),
        captured_package_documents(),
    )
}

fn one_shot_portable_request() -> PortableWorkspaceRequest {
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            PortableWorkspaceDocument::new(
                "composition.toml".to_owned(),
                include_bytes!("fixtures/cross-leaf.v1/one-shot/workspace/composition.toml")
                    .to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "pale-workspace.toml".to_owned(),
                include_bytes!("fixtures/cross-leaf.v1/one-shot/workspace/pale-workspace.toml")
                    .to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "workspace-resolution.lock.toml".to_owned(),
                include_bytes!(
                    "fixtures/cross-leaf.v1/one-shot/workspace/workspace-resolution.lock.toml"
                )
                .to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "workspace-universe.lock.toml".to_owned(),
                include_bytes!(
                    "fixtures/cross-leaf.v1/one-shot/workspace/workspace-universe.lock.toml"
                )
                .to_vec(),
            ),
        ],
    )
}

#[test]
fn portable_workspace_enumerates_exact_profile_contexts() {
    let api = PaleApi::new().expect("API initializes");
    let compilation = api
        .compile_portable_workspace(one_shot_portable_request())
        .expect("workspace fixture compiles");
    let contexts = compilation.profile_contexts();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].project_id(), "demo");
    assert_eq!(contexts[0].execution_workspace_id(), "default");
    assert_eq!(
        serde_json::to_value(&contexts).unwrap(),
        serde_json::json!([{
            "projectId": "demo",
            "executionWorkspaceId": "default"
        }])
    );
}

#[test]
fn one_shot_golden_is_recomputed_from_the_api_owner() {
    let expected = fixture("one-shot-journey.json");
    let api = PaleApi::new().expect("API initializes");
    let compilation = api
        .compile_one_shot(one_shot_request())
        .expect("fixture compiles");
    let summary = compilation.summary();
    let repositories = summary
        .repositories()
        .iter()
        .map(|r| r.repository_id())
        .collect::<Vec<_>>();
    assert_eq!(repositories, ["engine"]);
    let prior = OneShotPriorMaterialization::Unowned;
    let targets = api
        .one_shot_observation_targets(
            &compilation,
            "engine".to_owned(),
            OneShotPlanKind::Apply,
            &prior,
        )
        .unwrap();
    let baseline = targets
        .iter()
        .map(|target| {
            OneShotBaselineEntry::new(
                target.relative_path().to_owned(),
                OneShotBaselineObservation::Absent,
            )
        })
        .collect();
    let plan = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_owned(),
                OneShotPlanKind::Apply,
                prior,
                baseline,
            ),
        )
        .unwrap();
    assert_eq!(plan.deferred_effect_count(), 0);
    let actual = serde_json::json!({
        "capability_ids": ["one-shot.compilation", "one-shot.compilation-summary", "one-shot.observation-targets", "one-shot.planning"],
        "selectors": {"project_id": "demo", "execution_workspace_id": "default", "machine_id": "machine", "repository_id": "engine"},
        "summary": {
            "project_id": summary.project_id(),
            "execution_workspace_id": summary.execution_workspace_id(),
            "profile_digest": summary.profile_digest(),
            "instance_digest": summary.instance_digest(),
            "one_shot_compilation_digest": summary.one_shot_compilation_digest(),
            "schema_manifest_sha256": summary.schema_manifest_sha256(),
            "schema_set_sha256": summary.schema_set_sha256(),
            "repository_ids": repositories,
            "canonical_sha256": pale_core::sha256_hex(summary.canonical_bytes()),
            "canonical_length": summary.canonical_bytes().len()
        },
        "observation_targets": targets.iter().map(|target| target.relative_path()).collect::<Vec<_>>(),
        "plan": {
            "repository_id": plan.repository_id(),
            "plan_digest": plan.plan_digest(),
            "one_shot_compilation_digest": plan.one_shot_compilation_digest(),
            "operation_count": plan.operation_count(),
            "deferred_effect_count": plan.deferred_effect_count(),
            "canonical_sha256": pale_core::sha256_hex(plan.canonical_bytes()),
            "canonical_length": plan.canonical_bytes().len()
        }
    });
    conformance::assert_json("one-shot-journey.json", &actual, &expected);
}

#[test]
fn observation_golden_is_recomputed_from_the_api_owner() {
    let expected = fixture("observation-journey.json");
    let api = PaleApi::new().expect("API initializes");
    let workspace = api
        .compile_portable_workspace(one_shot_portable_request())
        .expect("workspace fixture compiles");
    let profile = api
        .compile_effective_profile(&workspace, effective_profile_request())
        .expect("effective profile fixture compiles");
    let snapshot = api.observe_effective_profile(&profile);

    let page_filter = ObservationFilter::all().with_category(ObservationCategory::Readiness);
    let page_limits = ObservationPageLimits::new(16, 1, 4_096);
    let page = snapshot
        .page(ObservationPageRequest::bounded(
            page_filter.clone(),
            page_limits,
            ObservationCancellation::Active,
        ))
        .expect("first page belongs to this snapshot");
    let ObservationPageDisposition::Ready { page: ready_page } = &page else {
        panic!("an active page request cannot be cancelled");
    };
    let continuation = ready_page
        .continuation
        .clone()
        .expect("the bounded page retains a continuation");
    let continuation_mismatch = snapshot
        .page(
            ObservationPageRequest::bounded(
                page_filter,
                ObservationPageLimits::new(16, 2, 4_096),
                ObservationCancellation::Active,
            )
            .after(continuation),
        )
        .expect_err("a continuation cannot cross raw limits");

    let all = snapshot
        .page(ObservationPageRequest::bounded(
            ObservationFilter::all(),
            ObservationPageLimits::new(256, 256, u32::MAX),
            ObservationCancellation::Active,
        ))
        .expect("complete discovery page belongs to this snapshot");
    let ObservationPageDisposition::Ready { page: all } = all else {
        panic!("an active discovery request cannot be cancelled");
    };
    let execution_path = all
        .facts
        .iter()
        .find_map(|fact| match fact {
            ObservationFact::Readiness { subject, .. }
                if subject.kind() == ObservationSubjectKind::ExecutionPath =>
            {
                Some(subject.clone())
            }
            _ => None,
        })
        .expect("the fixture retains one execution-path readiness fact");
    let causal_path = snapshot
        .causal_path(&ObservationCausalSelector::new(&snapshot, &execution_path))
        .expect("the selector belongs to this snapshot");
    let causal_kind = if causal_path.is_some() {
        "ready"
    } else {
        "absent"
    };

    let lens_limits = ObservationLensLimits::new(256, 1, 4_096, 1, 256);
    let lens = snapshot.visual_lens(ObservationLensRequest::bounded(
        ObservationFilter::all().with_subject(&execution_path),
        lens_limits,
        true,
        ObservationCancellation::Active,
    ));
    let page_projection = match &page {
        ObservationPageDisposition::Ready { page } => serde_json::json!({
            "kind": "ready",
            "page": {
                "cursor": page.cursor,
                "facts": page.facts,
                "continuation": page.continuation.as_ref().map(|cursor| serde_json::json!({
                    "snapshot": cursor.snapshot(),
                    "inspectedPosition": cursor.inspected_position(),
                })),
                "completion": page.completion,
                "inspected": page.inspected,
                "copiedStringBytes": page.copied_string_bytes,
                "snapshotOmittedCount": page.snapshot_omitted_count,
            }
        }),
        ObservationPageDisposition::Cancelled { .. } => {
            panic!("the active page request cannot be cancelled")
        }
    };
    let lens_projection = match &lens {
        ObservationLensDisposition::Ready { lens } => serde_json::json!({
            "kind": "ready",
            "lens": {
                "cursor": lens.cursor,
                "nodes": lens.nodes,
                "causalPaths": lens.causal_paths,
                "continuation": lens.continuation.as_ref().map(|cursor| serde_json::json!({
                    "snapshot": cursor.snapshot(),
                    "inspectedPosition": cursor.inspected_position(),
                })),
                "completion": lens.completion,
                "inspectedSubjects": lens.inspected_subjects,
                "copiedStringBytes": lens.copied_string_bytes,
                "causalPathLookups": lens.causal_path_lookups,
                "causalSteps": lens.causal_steps,
                "snapshotOmittedCount": lens.snapshot_omitted_count,
            }
        }),
        ObservationLensDisposition::Resynchronize { .. }
        | ObservationLensDisposition::Cancelled { .. } => {
            panic!("the active first lens step must be ready")
        }
    };

    let actual = serde_json::json!({
        "snapshot": snapshot.summary(),
        "pageLimits": page_limits,
        "page": page_projection,
        "continuationMismatch": {
            "kind": "resynchronize",
            "resynchronization": continuation_mismatch,
        },
        "causalPath": {
            "kind": causal_kind,
            "path": causal_path,
        },
        "lensLimits": lens_limits,
        "lens": lens_projection,
    });
    conformance::assert_json("observation-journey.json", &actual, &expected);
}

#[test]
fn repository_admission_fixture_is_recomputed_from_the_api_owner() {
    let api = PaleApi::new().expect("API initializes");
    let (catalog, descriptors) = repository_catalog::repository_catalog_fixture(&api);
    let actual = serde_json::to_value(&catalog).unwrap();
    let bytes = pale_core::canon::to_canonical_string(&catalog)
        .unwrap()
        .into_bytes();
    let admitted = api
        .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
            bytes,
            catalog.profile_digest.to_string(),
            catalog.instance_digest.to_string(),
            catalog.repository_id.to_string(),
        ))
        .expect("catalog admits");
    assert_eq!(admitted.descriptors(), descriptors);
    assert_eq!(descriptors.len(), 11);
    conformance::assert_json(
        "repository-catalog.json",
        &actual,
        &fixture("repository-catalog.json"),
    );
    conformance::assert_json(
        "repository-admission.json",
        &serde_json::json!({
            "capability_id": "native-action.repository-admission",
            "profile_digest": admitted.profile_digest(),
            "instance_digest": admitted.instance_digest(),
            "repository_id": admitted.repository_id()
        }),
        &fixture("repository-admission.json"),
    );
}

#[test]
fn native_action_execution_golden_is_recomputed_from_the_api_owner() {
    let expected = fixture("native-action-journey.json");
    let api = PaleApi::new().expect("API initializes");
    let descriptors = api.native_action_descriptors().expect("catalog projects");
    let tagged = api
        .invoke_native_action(NativeActionRequest::new(
            expected["action_id"].as_str().unwrap().to_owned(),
            expected["tagged_input"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec(),
        ))
        .expect("tagged action executes");
    let payload = api
        .invoke_native_action_payload(NativeActionPayloadRequest::new(
            expected["action_id"].as_str().unwrap().to_owned(),
            expected["payload_input"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec(),
        ))
        .expect("payload action executes");
    assert_eq!(tagged.canonical_bytes(), payload.canonical_bytes());
    // Retain the authored input seeds; only API-derived outputs are regenerated.
    let mut actual = expected.clone();
    actual["descriptor_ids"] = serde_json::to_value(
        descriptors
            .iter()
            .map(|d| d.action_id())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    actual["descriptor_projections"] = serde_json::json!(descriptors
        .iter()
        .map(|d| serde_json::json!([
            d.action_id(),
            d.description(),
            pale_core::sha256_hex(d.canonical_contract_bytes()),
            d.canonical_contract_bytes().len(),
            pale_core::sha256_hex(d.canonical_payload_schema_bytes()),
            d.canonical_payload_schema_bytes().len()
        ]))
        .collect::<Vec<_>>());
    actual["execution_sha256"] = pale_core::sha256_hex(payload.canonical_bytes()).into();
    actual["execution_length"] = payload.canonical_bytes().len().into();
    conformance::assert_json("native-action-journey.json", &actual, &expected);
}

#[test]
fn portable_journey_golden_is_recomputed_from_the_api_owner() {
    let expected = fixture("portable-journey.json");
    let documents = [
        (
            "pale-workspace.toml",
            include_bytes!("fixtures/cross-leaf.v1/portable/pale-workspace.toml").as_slice(),
        ),
        (
            "resolution.lock.toml",
            include_bytes!("fixtures/cross-leaf.v1/portable/resolution.lock.toml").as_slice(),
        ),
        (
            "universe.lock.toml",
            include_bytes!("fixtures/cross-leaf.v1/portable/universe.lock.toml").as_slice(),
        ),
    ];
    let request = || {
        PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_owned(),
            documents
                .iter()
                .map(|(label, bytes)| {
                    PortableWorkspaceDocument::new((*label).to_owned(), bytes.to_vec())
                })
                .collect(),
        )
    };
    let api = PaleApi::new().expect("API initializes");
    let limits = PaleApi::authoring_ingress_limits(AuthoringIngressClass::Workspace);
    let prior = api
        .compile_portable_workspace(request())
        .expect("prior admits");
    let current = api
        .compile_portable_workspace(request())
        .expect("current admits");
    assert_eq!(prior.admission_evidence_count(), 0);
    assert_eq!(prior.adopted_release_count(), 0);
    assert_eq!(prior.package_release_count(), 0);
    assert_eq!(prior.normative_pack_release_count(), 0);
    let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();
    assert_eq!(comparison.change_root_count(), 0);
    assert_eq!(comparison.affected_count(), 0);
    assert_eq!(comparison.unaffected_count(), 9);
    let roots = comparison.change_roots(0, u16::MAX);
    assert!(roots.entries().is_empty());
    assert_eq!(roots.next_offset(), None);
    let impacts = comparison.impacts(0, 1);
    assert_eq!(impacts.next_offset(), Some(1));
    let impact = &impacts.entries()[0];
    assert_eq!(
        impact.subject().kind(),
        PortableWorkspaceSubjectKind::Source
    );
    assert_eq!(impact.subject().source_id(), Some("resolution-lock"));
    assert_eq!(impact.state(), PortableWorkspaceImpactState::Unaffected);
    assert_eq!(impact.contributing_root_count(), 0);
    let explanation = comparison.explain(impact.subject()).unwrap();
    assert_eq!(
        explanation.state(),
        PortableWorkspaceExplanationState::Unaffected
    );
    assert_eq!(explanation.subject(), impact.subject());
    assert!(explanation.causal().is_none());
    let actual = serde_json::json!({
        "capability_ids": ["workspace.portable-compilation", "workspace.portable-comparison", "workspace.portable-change-roots", "workspace.portable-impacts", "workspace.portable-explanation"],
        "root_manifest": "pale-workspace.toml",
        "document_labels": documents.iter().map(|(label, _)| label).collect::<Vec<_>>(),
        "workspace_authoring_limits": {
            "max_documents": limits.max_documents(),
            "max_document_bytes": limits.max_document_bytes().to_string(),
            "max_total_bytes": limits.max_total_bytes().to_string()
        },
        "compilation": {
            "workspace_id": prior.workspace_id(),
            "principal_universe_digest": prior.principal_universe_digest(),
            "semantic_ir_digest": prior.semantic_ir_digest(),
            "admission_evidence_digest": prior.admission_evidence_digest(),
            "source_document_count": prior.source_document_count(),
            "derivation_source_count": prior.derivation_source_count(),
            "derivation_node_count": prior.derivation_node_count(),
            "admission_evidence_count": prior.admission_evidence_count(),
            "adopted_release_count": prior.adopted_release_count(),
            "package_release_count": prior.package_release_count(),
            "normative_pack_release_count": prior.normative_pack_release_count()
        },
        "comparison": {
            "prior_principal_universe_digest": comparison.prior_principal_universe_digest(),
            "current_principal_universe_digest": comparison.current_principal_universe_digest(),
            "prior_semantic_ir_digest": comparison.prior_semantic_ir_digest(),
            "current_semantic_ir_digest": comparison.current_semantic_ir_digest(),
            "receipt_digest": comparison.receipt_digest(),
            "change_root_count": comparison.change_root_count(),
            "affected_count": comparison.affected_count(),
            "unaffected_count": comparison.unaffected_count()
        },
        "change_roots": {"entry_count": roots.entries().len(), "next_offset": roots.next_offset()},
        "first_impact": {"next_offset": impacts.next_offset(), "kind": "source", "source_id": impact.subject().source_id(), "state": "unaffected", "contributing_root_count": impact.contributing_root_count()},
        "explanation": {"state": "unaffected", "frontier": null, "step_count": 0}
    });
    conformance::assert_json("portable-journey.json", &actual, &expected);
}

#[test]
fn bundle_identity_golden_is_recomputed_from_the_api_owner() {
    let api = PaleApi::new().expect("API initializes");
    let identity = api.contract_bundle_identity();
    conformance::assert_json(
        "bundle-identity.json",
        &serde_json::json!({
            "capability_id": "contract.bundle-identity",
            "member_count": identity.member_count(),
            "schema_manifest_sha256": identity.schema_manifest_sha256(),
            "schema_set_sha256": identity.schema_set_sha256()
        }),
        &fixture("bundle-identity.json"),
    );
}

#[test]
fn contract_and_diagnostic_golden_is_recomputed_from_the_api_owner() {
    let expected = fixture("contract-diagnostic.json");
    assert_eq!(
        expected["capability_ids"],
        serde_json::json!([
            "contract.schema-lookup",
            "contract.document-validation",
            "diagnostic.projection",
            "diagnostic.admission"
        ])
    );
    let api = PaleApi::new().expect("API initializes");
    let schema = api
        .contract_schema(expected["schema"]["registry_id"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        pale_core::sha256_hex(schema.bytes()),
        schema.schema_sha256()
    );
    let projected = api
        .project_diagnostic(DiagnosticProjection::new(
            expected["projection"]["code"].as_str().unwrap().to_owned(),
            DiagnosticStage::InputAdmission,
            DiagnosticDisposition::InvalidInput,
            expected["projection"]["summary"]
                .as_str()
                .unwrap()
                .to_owned(),
            DiagnosticRetry::AfterCorrection,
            DiagnosticEffectCertainty::NotApplicable,
        ))
        .unwrap();
    let outcome_unknown = api
        .project_diagnostic(DiagnosticProjection::new(
            expected["outcome_unknown"]["code"]
                .as_str()
                .unwrap()
                .to_owned(),
            DiagnosticStage::Materialization,
            DiagnosticDisposition::EffectOutcomeUnknown,
            expected["outcome_unknown"]["summary"]
                .as_str()
                .unwrap()
                .to_owned(),
            DiagnosticRetry::ReconcileFirst,
            DiagnosticEffectCertainty::OutcomeUnknown,
        ))
        .unwrap();
    let admitted = api.admit_diagnostic(projected.canonical_bytes()).unwrap();
    assert_eq!(admitted.canonical_bytes(), projected.canonical_bytes());
    let validated = api
        .validate_contract_document(ContractDocument::new(
            schema.registry_id().to_owned(),
            projected.canonical_bytes().to_vec(),
        ))
        .unwrap();
    assert_eq!(validated.schema_id(), "diagnostic.v1");
    assert_eq!(validated.schema_sha256(), schema.schema_sha256());
    assert_eq!(validated.canonical_bytes(), projected.canonical_bytes());
    let mut actual = expected.clone();
    actual["schema"] = serde_json::json!({
        "registry_id": schema.registry_id(), "schema_uri": schema.schema_uri(),
        "wire_schema_id": schema.wire_schema_id(), "schema_sha256": schema.schema_sha256(),
        "byte_length": schema.bytes().len()
    });
    actual["canonical_diagnostic"] = projected.canonical_json().into();
    actual["outcome_unknown"]["canonical_diagnostic"] = outcome_unknown.canonical_json().into();
    conformance::assert_json("contract-diagnostic.json", &actual, &expected);
}
