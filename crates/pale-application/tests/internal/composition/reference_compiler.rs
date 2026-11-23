//! Exercises reference composition across every admitted graph and ownership boundary.

use std::collections::BTreeMap;

use pale_core::records::{
    CompositionEdgeV1, CompositionGraphKindV1, CompositionReasonStepV1, ContinuityIdentityV1,
    DeploymentActivationIdentityV1, DestinationConsequenceV1, EvidenceSettlementIdentityV1,
    ExecutionPathSelectionIdV1, ExecutionPathSelectionV1, ExecutionPathWitnessV1, ExperiencePathV1,
    InvalidExecutionPathWitnessV1, InvocationIdentityV1, NativeDataGenerationIdentityV1,
    NativeEffectIdentityV1, NativeOutcomeV1, NativeOwnerTransitionV1,
    PaleAdmissionGenerationIdentityV1, ParticipantProviderManifestV1, ParticipantRefV1,
    PathInvocationV1, ProviderBindingIdentityV1, ProviderCompatibilityDeclarationV1,
    ReconciliationRequirementV1, RequestTranslationV1, ResponsibilityContractV1,
    ResponsibilityPathIdentityV1, ResponsibilityV1, ResultTranslationV1, SchemaDigest,
};

use super::provider::{compile_provider_components, ProviderComponents};
use super::*;

use crate::composition_source_fixtures::{
    budget, engine, neutral_composition_declarations, participant,
    populated_composition_declarations,
};
use crate::provider_binding_fixtures::{
    contract_schema_identity, digest, provider_binding, provider_binding_with_coordinates,
};

fn providers_are_equivalent(components: &ProviderComponents<'_>, left: &str, right: &str) -> bool {
    let position = |provider: &str| {
        components
            .providers
            .binary_search(&provider)
            .expect("test provider belongs to the compiled partition")
    };
    components.component_of[position(left)] == components.component_of[position(right)]
}

fn neutral_source() -> CompositionSourceV1 {
    neutral_composition_declarations()
}

fn direct_source() -> CompositionSourceV1 {
    let mut source = neutral_source();
    let entry = source.entry_contracts[0].entry.clone();
    source.termination_contracts[0].owner = entry;
    let mut path = source.selected_experience_paths[0].to_preimage();
    path.work_delegation_edge_ids.clear();
    source.selected_experience_paths[0] =
        ExperiencePathV1::seal(path).expect("direct experience path reseals");
    source
}

fn path_selection(plan: &ExperienceCompositionPlanV1) -> ExecutionPathSelectionV1 {
    ExecutionPathSelectionV1 {
        id: ExecutionPathSelectionIdV1::parse("selected-codex").unwrap(),
        experience_path_id: plan.declarations.selected_experience_paths[0].id.clone(),
    }
}

fn two_hop_work_source() -> CompositionSourceV1 {
    let mut source = neutral_source();
    let worker = participant("worker");
    let finisher = participant("finisher");
    source.participant_provider_manifests[0]
        .participants
        .push(finisher.clone());
    source
        .responsibility_contracts
        .push(ResponsibilityContractV1 {
            id: "responsibility-worker".into(),
            participant: worker.clone(),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        });
    source.request_translations.push(RequestTranslationV1 {
        id: "request-worker-finisher".into(),
        from: worker.clone(),
        to: finisher.clone(),
        input_contract: "worker-input".into(),
        output_contract: "finisher-input".into(),
        preserves_authority: true,
    });
    source.result_translations.push(ResultTranslationV1 {
        id: "result-finisher-worker".into(),
        from: finisher.clone(),
        to: worker.clone(),
        input_contract: "finisher-result".into(),
        output_contract: "worker-result".into(),
        preserves_evidence: true,
    });
    let mut second_edge_budget = budget();
    second_edge_budget.max_time_ms = 10;
    second_edge_budget.max_cost_units = 10;
    second_edge_budget.max_resource_units = 10;
    second_edge_budget.max_effect_count = 1;
    source
        .edge_contracts
        .push(CompositionEdgeV1::WorkDelegation {
            id: "delegation-worker-finisher".into(),
            from: worker,
            to: finisher.clone(),
            responsibility_contract_id: "responsibility-worker".into(),
            request_translation_id: "request-worker-finisher".into(),
            result_translation_id: "result-finisher-worker".into(),
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
            budget: second_edge_budget,
        });
    let path = &mut source.selected_experience_paths[0];
    path.work_delegation_edge_ids
        .push("delegation-worker-finisher".into());
    path.responsibility_contract_ids
        .push("responsibility-worker".into());
    source.termination_contracts[0].owner = finisher;
    source
}

#[test]
fn shared_populated_fixture_compiles_every_operational_and_view_edge_family() {
    let plan = compile(populated_composition_declarations()).expect("populated fixture compiles");
    assert_eq!(plan.delegation_graph.edges, ["delegation-entry-worker"]);
    assert_eq!(plan.terminal_leaf_graph.edges, ["terminal-worker-output"]);
    assert_eq!(
        plan.evidence_settlement_graph.edges,
        ["observe-entry", "observe-entry-again"]
    );
    assert_eq!(plan.ui_embedding_graph.edges, ["ui-entry-worker"]);
    assert_eq!(plan.plugin_dependency_graph.edges, ["plugin-worker-entry"]);
}

#[test]
fn participant_provider_manifest_rejects_authored_monotonicity_claims() {
    let manifest = neutral_source()
        .participant_provider_manifests
        .into_iter()
        .next()
        .expect("fixture carries one participant-provider manifest");
    let mut value = serde_json::to_value(manifest).expect("serialize manifest fixture");
    value["providers"][0]["monotone"] = serde_json::json!(true);

    let error = serde_json::from_value::<ParticipantProviderManifestV1>(value)
        .expect_err("compiler-derived reciprocity cannot be authored as a Boolean claim");
    assert!(error.to_string().contains("unknown field `monotone`"));
}

fn native_invocation_chain_source() -> CompositionSourceV1 {
    let mut source = neutral_source();
    let entry = participant("entry");
    let application_a = ParticipantRefV1::Application {
        application_id: "application-a".into(),
    };
    let application_b = ParticipantRefV1::Application {
        application_id: "application-b".into(),
    };
    source.participant_provider_manifests[0]
        .participants
        .extend([application_a.clone(), application_b.clone()]);
    source
        .responsibility_contracts
        .push(ResponsibilityContractV1 {
            id: "responsibility-application-a".into(),
            participant: application_a.clone(),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        });
    let mut application_binding = provider_binding(application_a.clone());
    application_binding.id = "provider-binding-application-a".into();
    application_binding.binding_identity = ProviderBindingIdentityV1 {
        provider_binding_sha256: digest("provider-binding-application-a"),
    };
    application_binding.responsibility_contract_ids = vec!["responsibility-application-a".into()];
    source.provider_bindings.push(application_binding);
    source.request_translations.extend([
        RequestTranslationV1 {
            id: "request-native-a".into(),
            from: entry.clone(),
            to: application_a.clone(),
            input_contract: "native-a-input".into(),
            output_contract: "application-a-input".into(),
            preserves_authority: true,
        },
        RequestTranslationV1 {
            id: "request-native-b".into(),
            from: application_a.clone(),
            to: application_b.clone(),
            input_contract: "native-b-input".into(),
            output_contract: "application-b-input".into(),
            preserves_authority: true,
        },
    ]);
    source.result_translations.extend([
        ResultTranslationV1 {
            id: "result-native-a".into(),
            from: application_a.clone(),
            to: entry.clone(),
            input_contract: "application-a-result".into(),
            output_contract: "native-a-result".into(),
            preserves_evidence: true,
        },
        ResultTranslationV1 {
            id: "result-native-b".into(),
            from: application_b.clone(),
            to: application_a.clone(),
            input_contract: "application-b-result".into(),
            output_contract: "native-b-result".into(),
            preserves_evidence: true,
        },
    ]);
    source.edge_contracts.extend([
        CompositionEdgeV1::NativeApplicationCall {
            id: "native-call-a".into(),
            caller: entry,
            application: application_a.clone(),
            provider_binding_id: "provider-binding-a".into(),
            request_translation_id: "request-native-a".into(),
            result_translation_id: "result-native-a".into(),
        },
        CompositionEdgeV1::NativeApplicationCall {
            id: "native-call-b".into(),
            caller: application_a,
            application: application_b,
            provider_binding_id: "provider-binding-application-a".into(),
            request_translation_id: "request-native-b".into(),
            result_translation_id: "result-native-b".into(),
        },
    ]);
    source.selected_experience_paths[0]
        .responsibility_contract_ids
        .push("responsibility-application-a".into());
    source.selected_experience_paths[0]
        .provider_binding_ids
        .push("provider-binding-application-a".into());
    source.selected_experience_paths[0].invocations = vec![
        PathInvocationV1 {
            edge_id: "native-call-a".into(),
            identity: InvocationIdentityV1 {
                invocation_sha256: digest("native-call-a"),
            },
        },
        PathInvocationV1 {
            edge_id: "native-call-b".into(),
            identity: InvocationIdentityV1 {
                invocation_sha256: digest("native-call-b"),
            },
        },
    ];
    source
}

fn fully_populated_path_source() -> CompositionSourceV1 {
    let mut source = native_invocation_chain_source();
    let application = ParticipantRefV1::Application {
        application_id: "application-a".into(),
    };
    let invocation_identity = InvocationIdentityV1 {
        invocation_sha256: digest("native-call-a"),
    };
    let path = &mut source.selected_experience_paths[0];
    path.native_owner_transitions = vec![NativeOwnerTransitionV1 {
        edge_id: "native-call-a".into(),
        native_owner_id: "native-owner-a".into(),
        transition_contract_id: "native-transition-a".into(),
        receipt_contract_id: "native-receipt-a".into(),
    }];
    path.native_outcomes = vec![NativeOutcomeV1::EffectfulOutcome {
        edge_id: "native-call-a".into(),
        invocation_identity,
        native_effect_identity: NativeEffectIdentityV1 {
            native_effect_sha256: digest("native-effect-a"),
        },
        outcome_contract_id: "native-outcome-a".into(),
    }];
    path.destination_consequences = vec![DestinationConsequenceV1 {
        destination: application,
        consequence_contract_id: "destination-consequence-a".into(),
    }];
    path.reconciliation = vec![ReconciliationRequirementV1 {
        obligation_contract_id: "reconciliation-obligation-a".into(),
        frontier_id: "source-frontier-a".into(),
    }];
    source.settlement_contracts[0].native_receipt_contract = Some("native-receipt-a".into());
    source
}

fn selected_identity_axes(plan: &ExperienceCompositionPlanV1) -> Vec<serde_json::Value> {
    let value = serde_json::to_value(plan).expect("serialize compiled composition plan");
    [
            "/declarations/selected_experience_paths/0/identity/responsibility_path_sha256",
            "/declarations/provider_bindings/0/binding_identity/provider_binding_sha256",
            "/declarations/provider_bindings/0/generation_coordinates/deployment_activation_identity/deployment_activation_sha256",
            "/declarations/selected_experience_paths/0/invocations/0/identity/invocation_sha256",
            "/declarations/selected_experience_paths/0/native_outcomes/0/native_effect_identity/native_effect_sha256",
            "/declarations/settlement_contracts/0/identity/evidence_settlement_sha256",
            "/declarations/continuity_contracts/0/identity/continuity_sha256",
            "/declarations/provider_bindings/0/generation_coordinates/native_data_generation_identity/native_data_generation_sha256",
            "/declarations/provider_bindings/0/generation_coordinates/pale_admission_generation_identity/pale_admission_generation_sha256",
        ]
        .into_iter()
        .map(|pointer| {
            value
                .pointer(pointer)
                .unwrap_or_else(|| panic!("compiled plan omits identity axis `{pointer}`"))
                .clone()
        })
        .collect()
}

fn reseal_source(mut source: CompositionSourceV1) -> CompositionSourceV1 {
    let mut normalized = source.clone();
    if normalize::normalize_source(&mut normalized).is_ok() {
        source = normalized;
    }
    for binding in &mut source.provider_bindings {
        binding
            .generation_coordinates
            .deployment_activation_identity = binding
            .recompute_deployment_activation_identity()
            .expect("test deployment coordinates are canonical");
        binding.binding_identity = binding
            .recompute_identity()
            .expect("test provider bindings are canonical");
    }
    for path in &mut source.selected_experience_paths {
        path.identity = path
            .recompute_identity()
            .expect("test experience paths are canonical");
    }
    source
}

fn compile(source: CompositionSourceV1) -> Result<ExperienceCompositionPlanV1, CompositionError> {
    compile_claims(reseal_source(source))
}

fn selection(id: &str, experience_path_id: &str) -> ExecutionPathSelectionV1 {
    ExecutionPathSelectionV1 {
        id: ExecutionPathSelectionIdV1::parse(id).expect("test selection id is portable"),
        experience_path_id: experience_path_id.into(),
    }
}

// Most graph laws inspect their cause. Phase laws below retain the outer carrier.
fn compile_claims(
    source: CompositionSourceV1,
) -> Result<ExperienceCompositionPlanV1, CompositionError> {
    compile_composition_plan(&[selection("codex-neutral", "neutral-path")], source).map_err(
        |failure| match failure {
            CompositionPlanFailure::Declaration(source)
            | CompositionPlanFailure::GeneratedOutput(source) => source,
        },
    )
}

#[test]
fn authored_binding_rejection_and_generated_record_failure_keep_their_phases() {
    let selections = [selection("codex-neutral", "neutral-path")];
    let mut authored = neutral_source();
    authored.provider_bindings[0]
        .artifact_coordinates
        .executable = "changed-executable".into();
    assert!(matches!(
        compile_composition_plan(&selections, authored),
        Err(CompositionPlanFailure::Declaration(
            CompositionError::ProviderBinding {
                violation: ProviderBindingViolation::BindingIdentityMismatch,
                ..
            }
        ))
    ));

    let normalized = NormalizedComposition::admit(&selections, neutral_source()).unwrap();
    let mut linked = LinkedComposition::link(normalized).unwrap();
    // Fault injection after successful linking exercises the actual output seal,
    // not a second compiler or a caller-input shortcut.
    linked.normalized.source.responsibility_contracts[0]
        .id
        .clear();
    assert!(matches!(
        linked.seal(),
        Err(CompositionPlanFailure::GeneratedOutput(
            CompositionError::Schema(pale_contracts::SchemaError::Invalid(_))
        ))
    ));
}

#[test]
fn graph_01_and_02_keep_typed_participants_and_explicit_entry_origin() {
    let plan = compile(neutral_source()).unwrap();
    let entry = serde_json::to_value(&plan.declarations.entry_contracts[0]).unwrap();
    assert_eq!(entry["entry"]["kind"], "capability");
    assert_eq!(entry["origin"]["kind"], "user");
    for inferred in ["caller", "callee", "native_owner"] {
        assert!(entry.get(inferred).is_none());
        assert!(entry["entry"].get(inferred).is_none());
    }
    assert_ne!(engine("shared"), participant("shared"));
}

#[test]
fn graph_03_preserves_direction_contracts_ceiling_and_budget() {
    let plan = compile(neutral_source()).unwrap();
    assert_eq!(plan.delegation_graph.edges, ["delegation-entry-worker"]);
    assert_eq!(plan.declarations.provider_bindings.len(), 1);
    assert_eq!(
        plan.declarations.selected_experience_paths[0].provider_binding_ids,
        ["provider-binding-a"]
    );
    let CompositionEdgeV1::WorkDelegation {
        from,
        to,
        request_translation_id,
        result_translation_id,
        authority_ceiling,
        budget,
        ..
    } = &plan.declarations.edge_contracts[0]
    else {
        panic!("fixture edge must remain a work delegation");
    };
    assert_eq!(from, &participant("entry"));
    assert_eq!(to, &participant("worker"));
    assert_eq!(request_translation_id, "request-entry-worker");
    assert_eq!(result_translation_id, "result-worker-entry");
    assert_eq!(authority_ceiling, &[pale_core::records::EffectClass::Read]);
    assert_eq!(budget.max_time_ms, 10);
}

#[test]
fn execution_path_witness_projects_exact_identity_order_and_directness() {
    let direct_plan = compile(direct_source()).expect("direct path compiles");
    let direct =
        ExecutionPathWitnessV1::from_compiled_plan(&path_selection(&direct_plan), &direct_plan)
            .expect("direct witness projects");
    assert!(direct.is_direct());
    assert_eq!(
        direct.responsibility_path_identity,
        direct_plan.declarations.selected_experience_paths[0].identity
    );
    assert_eq!(direct.composition_plan_digest, direct_plan.plan_digest);

    let delegated_plan = compile(two_hop_work_source()).expect("delegated path compiles");
    let delegated = ExecutionPathWitnessV1::from_compiled_plan(
        &path_selection(&delegated_plan),
        &delegated_plan,
    )
    .expect("delegated witness projects");
    assert!(!delegated.is_direct());
    assert_eq!(
        delegated.work_delegation_edge_ids,
        ["delegation-entry-worker", "delegation-worker-finisher"]
    );
}

#[test]
fn execution_path_witness_rejects_stale_or_unknown_plan_claims() {
    let plan = compile(neutral_source()).expect("neutral path compiles");
    let selection = path_selection(&plan);

    let mut stale_plan = plan.clone();
    stale_plan.declarations.selected_experience_paths[0].privacy_policy_id =
        "changed-policy".into();
    assert!(matches!(
        ExecutionPathWitnessV1::from_compiled_plan(&selection, &stale_plan),
        Err(InvalidExecutionPathWitnessV1::CompositionPlanDigestMismatch)
    ));

    let mut unknown = selection.clone();
    unknown.experience_path_id = "missing-path".into();
    assert!(matches!(
        ExecutionPathWitnessV1::from_compiled_plan(&unknown, &plan),
        Err(InvalidExecutionPathWitnessV1::UnknownExperiencePath { .. })
    ));

    let mut stale_path_body = plan.to_preimage();
    stale_path_body.declarations.selected_experience_paths[0].privacy_policy_id =
        "changed-policy".into();
    let stale_path_plan = ExperienceCompositionPlanV1::seal(stale_path_body).unwrap();
    assert!(matches!(
        ExecutionPathWitnessV1::from_compiled_plan(&selection, &stale_path_plan),
        Err(InvalidExecutionPathWitnessV1::ResponsibilityPathIdentityMismatch { .. })
    ));
}

#[test]
fn admitted_path_proof_is_exact_ordered_reason_projection() {
    let source = two_hop_work_source();
    let plan = compile(source.clone()).expect("the two-hop path is admitted");
    assert_eq!(plan.reason_paths.len(), 1);
    assert_eq!(
        plan.reason_paths[0].reasons,
        [
            CompositionReasonStepV1::OriginReceipt {
                contract_id: "origin-receipt".into(),
            },
            CompositionReasonStepV1::EntryContract {
                contract_id: "entry-contract".into(),
            },
            CompositionReasonStepV1::ResponsibilityContract {
                contract_id: "responsibility-entry".into(),
            },
            CompositionReasonStepV1::ResponsibilityContract {
                contract_id: "responsibility-worker".into(),
            },
            CompositionReasonStepV1::ProviderBinding {
                binding_id: "provider-binding-a".into(),
            },
            CompositionReasonStepV1::DelegationEdge {
                edge_id: "delegation-entry-worker".into(),
            },
            CompositionReasonStepV1::DelegationEdge {
                edge_id: "delegation-worker-finisher".into(),
            },
            CompositionReasonStepV1::TerminationContract {
                contract_id: "termination-worker".into(),
            },
            CompositionReasonStepV1::SettlementContract {
                contract_id: "settlement-worker".into(),
            },
            CompositionReasonStepV1::ContinuityContract {
                contract_id: "continuity-entry".into(),
            },
        ]
    );
    assert_eq!(
        plan.reason_paths[0].path_id,
        plan.declarations.selected_experience_paths[0].id
    );
    assert_eq!(
        plan.plan_digest.as_str(),
        "a37df5e0336e38f8048897d6ff65be00d7250978350cef09c2384338d18fb4f9"
    );
    assert_eq!(
        pale_core::canon::digest(&plan).expect("the admitted plan has canonical JSON"),
        "2ecdff2b4f6109d355a71e6dc8cf853628a9985bfba1cb2b6ebdb6b725c911c5"
    );

    let mut permuted = source.clone();
    permuted.responsibility_contracts.reverse();
    permuted.request_translations.reverse();
    permuted.result_translations.reverse();
    permuted.edge_contracts.reverse();
    permuted.participant_provider_manifests[0]
        .participants
        .reverse();
    permuted.selected_experience_paths[0]
        .responsibility_contract_ids
        .reverse();
    assert_eq!(plan, compile(permuted).expect("set-like input normalizes"));

    let mut transposed = source;
    transposed.selected_experience_paths[0]
        .work_delegation_edge_ids
        .reverse();
    assert!(matches!(
        compile(transposed),
        Err(CompositionError::Path { reason, .. })
            if reason.contains("does not continue from")
    ));
}

#[test]
fn failed_path_cannot_emit_a_valid_prefix_proof() {
    let mut source = neutral_source();
    source
        .responsibility_contracts
        .push(ResponsibilityContractV1 {
            id: "responsibility-unused".into(),
            participant: participant("worker"),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        });
    let mut invalid = source.selected_experience_paths[0].clone();
    invalid.id = "z-invalid-path".into();
    invalid
        .responsibility_contract_ids
        .push("responsibility-unused".into());
    source.selected_experience_paths.push(invalid);

    assert!(matches!(
        compile(source),
        Err(CompositionError::Path { path, reason })
            if path == "z-invalid-path"
                && reason == "selected responsibility contracts do not exactly match the connected path closure"
    ));
}

#[test]
fn graph_04_requires_reciprocal_compatibility_without_creating_a_reverse_call() {
    let plan = compile(neutral_source()).unwrap();
    assert_eq!(plan.provider_compatibility_graph.edges.len(), 2);
    assert_eq!(plan.delegation_graph.edges.len(), 1);

    let mut asymmetric = neutral_source();
    asymmetric.participant_provider_manifests[0].providers[0]
        .compatible_provider_ids
        .clear();
    assert!(matches!(
        compile(asymmetric),
        Err(CompositionError::ProviderCompatibility { .. })
    ));
}

#[test]
fn graph_05_is_deterministic_bounded_and_classifies_cycles_by_relation() {
    let source = neutral_source();
    let first = compile(source.clone()).unwrap();
    let mut permuted = source.clone();
    permuted.participant_provider_manifests.reverse();
    permuted.participant_provider_manifests[0]
        .participants
        .reverse();
    permuted.participant_provider_manifests[0]
        .providers
        .reverse();
    permuted.termination_contracts[0].kinds.reverse();
    assert_eq!(first, compile(permuted).unwrap());

    let providers = source.participant_provider_manifests[0]
        .providers
        .iter()
        .collect::<Vec<_>>();
    let by_id = providers
        .iter()
        .map(|provider| (provider.id.as_str(), *provider))
        .collect::<BTreeMap<_, _>>();
    let components = compile_provider_components(&providers, &by_id).unwrap();
    assert!(providers_are_equivalent(
        &components,
        "provider-a",
        "provider-b"
    ));

    let mut provider_scc = source.clone();
    let manifest = &mut provider_scc.participant_provider_manifests[0];
    let provider_c_participant = engine("provider-c-participant");
    manifest.participants.push(provider_c_participant.clone());
    manifest.providers[0]
        .compatible_provider_ids
        .push("provider-c".into());
    manifest.providers.push(ProviderCompatibilityDeclarationV1 {
        id: "provider-c".into(),
        participant: provider_c_participant,
        capability_ids: vec!["neutral-capability".into()],
        compatible_provider_ids: vec!["provider-b".into()],
    });
    let providers = manifest.providers.iter().collect::<Vec<_>>();
    let by_id = providers
        .iter()
        .map(|provider| (provider.id.as_str(), *provider))
        .collect::<BTreeMap<_, _>>();
    let components = compile_provider_components(&providers, &by_id).unwrap();
    assert!(providers_are_equivalent(
        &components,
        "provider-a",
        "provider-c"
    ));

    let provider_scc = compile(provider_scc).unwrap();
    assert_eq!(provider_scc.provider_compatibility_graph.edges.len(), 4);
    assert!(!provider_scc
        .provider_compatibility_graph
        .edges
        .iter()
        .any(|edge| edge == "engine:provider-a-participant->engine:provider-c-participant"));

    let mut over_budget = source.clone();
    over_budget.selected_experience_paths[0].budget.max_time_ms = 5;
    assert!(matches!(
        compile(over_budget),
        Err(CompositionError::Path { .. })
    ));

    let mut excessive = source.clone();
    excessive.selected_experience_paths =
        vec![source.selected_experience_paths[0].clone(); MAX_SELECTED_PATHS + 1];
    assert!(matches!(
        compile(excessive),
        Err(CompositionError::DeclarationLimit { .. })
    ));

    let mut cartesian = source.clone();
    cartesian.participant_provider_manifests[0].providers[0].compatible_provider_ids =
        vec!["provider-b".into(); MAX_SEMANTIC_DECLARATIONS + 1];
    assert!(matches!(
        compile(cartesian),
        Err(CompositionError::DeclarationLimit { .. })
    ));

    let mut cyclic = source;
    cyclic
        .responsibility_contracts
        .push(ResponsibilityContractV1 {
            id: "responsibility-worker".into(),
            participant: participant("worker"),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        });
    cyclic.request_translations.push(RequestTranslationV1 {
        id: "request-worker-entry".into(),
        from: participant("worker"),
        to: participant("entry"),
        input_contract: "worker-input".into(),
        output_contract: "neutral-input".into(),
        preserves_authority: true,
    });
    cyclic.result_translations.push(ResultTranslationV1 {
        id: "result-entry-worker".into(),
        from: participant("entry"),
        to: participant("worker"),
        input_contract: "neutral-result".into(),
        output_contract: "worker-result".into(),
        preserves_evidence: true,
    });
    cyclic
        .edge_contracts
        .push(CompositionEdgeV1::WorkDelegation {
            id: "delegation-worker-entry".into(),
            from: participant("worker"),
            to: participant("entry"),
            responsibility_contract_id: "responsibility-worker".into(),
            request_translation_id: "request-worker-entry".into(),
            result_translation_id: "result-entry-worker".into(),
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
            budget: budget(),
        });
    cyclic.selected_experience_paths[0]
        .work_delegation_edge_ids
        .push("delegation-worker-entry".into());
    cyclic.selected_experience_paths[0]
        .responsibility_contract_ids
        .push("responsibility-worker".into());
    cyclic.termination_contracts[0].owner = participant("entry");
    assert!(matches!(
        compile(cyclic),
        Err(CompositionError::Path { .. })
    ));
}

#[test]
fn graph_06_keeps_all_seven_graph_domains_separate() {
    let plan = compile(neutral_source()).unwrap();
    assert_eq!(
        plan.provider_compatibility_graph.kind,
        CompositionGraphKindV1::ProviderCompatibility
    );
    assert_eq!(
        plan.delegation_graph.kind,
        CompositionGraphKindV1::WorkDelegation
    );
    assert_eq!(
        plan.ui_embedding_graph.kind,
        CompositionGraphKindV1::UiEmbedding
    );
    assert_eq!(
        plan.plugin_dependency_graph.kind,
        CompositionGraphKindV1::PluginDependency
    );
    assert_eq!(
        plan.native_application_graph.kind,
        CompositionGraphKindV1::NativeApplicationCall
    );
    assert_eq!(
        plan.terminal_leaf_graph.kind,
        CompositionGraphKindV1::TerminalLeaf
    );
    assert_eq!(
        plan.evidence_settlement_graph.kind,
        CompositionGraphKindV1::ObservationSettlement
    );
    assert!(plan.ui_embedding_graph.edges.is_empty());
    assert!(plan.plugin_dependency_graph.edges.is_empty());
    assert!(plan.native_application_graph.edges.is_empty());
}

#[test]
fn graph_10_preserves_intrinsic_edge_kinds_parallel_ids_and_non_authorizing_embedding() {
    let mut source = neutral_source();
    let plugin = ParticipantRefV1::Plugin {
        plugin_id: "plugin-a".into(),
    };
    let dependency = ParticipantRefV1::Plugin {
        plugin_id: "plugin-b".into(),
    };
    let application = ParticipantRefV1::Application {
        application_id: "application-a".into(),
    };
    let interface_host = ParticipantRefV1::Interface {
        interface_id: "interface-host".into(),
    };
    let interface_embedded = ParticipantRefV1::Interface {
        interface_id: "interface-embedded".into(),
    };
    let terminal_leaf = ParticipantRefV1::TerminalLeaf {
        terminal_leaf_id: "terminal-leaf-a".into(),
    };
    source.participant_provider_manifests[0]
        .participants
        .extend([
            plugin.clone(),
            dependency.clone(),
            application.clone(),
            interface_host.clone(),
            interface_embedded.clone(),
            terminal_leaf.clone(),
        ]);
    source.request_translations.push(RequestTranslationV1 {
        id: "request-entry-application".into(),
        from: participant("entry"),
        to: application.clone(),
        input_contract: "neutral-input".into(),
        output_contract: "application-input".into(),
        preserves_authority: true,
    });
    source.result_translations.push(ResultTranslationV1 {
        id: "result-application-entry".into(),
        from: application.clone(),
        to: participant("entry"),
        input_contract: "application-result".into(),
        output_contract: "neutral-result".into(),
        preserves_evidence: true,
    });
    source.termination_contracts[0].terminal_leaf = Some(terminal_leaf.clone());
    source.edge_contracts.extend([
        CompositionEdgeV1::UiEmbedding {
            id: "ui-embedding-a".into(),
            host: interface_host.clone(),
            embedded: interface_embedded.clone(),
        },
        CompositionEdgeV1::UiEmbedding {
            id: "ui-embedding-b".into(),
            host: interface_host,
            embedded: interface_embedded,
        },
        CompositionEdgeV1::PluginDependency {
            id: "plugin-dependency".into(),
            plugin,
            dependency,
        },
        CompositionEdgeV1::NativeApplicationCall {
            id: "native-call".into(),
            caller: participant("entry"),
            application: application.clone(),
            provider_binding_id: "provider-binding-a".into(),
            request_translation_id: "request-entry-application".into(),
            result_translation_id: "result-application-entry".into(),
        },
        CompositionEdgeV1::TerminalLeaf {
            id: "terminal-leaf".into(),
            owner: participant("worker"),
            terminal_leaf,
            termination_contract_id: "termination-worker".into(),
        },
        CompositionEdgeV1::ObservationSettlement {
            id: "observation-settlement".into(),
            subject: application,
            observer: participant("worker"),
            settlement_contract_id: "settlement-worker".into(),
        },
    ]);

    let plan = compile(source.clone()).unwrap();
    assert_eq!(plan.delegation_graph.edges, ["delegation-entry-worker"]);
    assert_eq!(
        plan.ui_embedding_graph.edges,
        ["ui-embedding-a", "ui-embedding-b"]
    );
    assert_eq!(plan.plugin_dependency_graph.edges, ["plugin-dependency"]);
    assert_eq!(plan.native_application_graph.edges, ["native-call"]);
    assert_eq!(plan.terminal_leaf_graph.edges, ["terminal-leaf"]);
    assert_eq!(
        plan.evidence_settlement_graph.edges,
        ["observation-settlement"]
    );

    let mut embedding_only = source;
    embedding_only
        .edge_contracts
        .retain(|edge| matches!(edge, CompositionEdgeV1::UiEmbedding { .. }));
    embedding_only.selected_experience_paths.clear();
    let embedding_plan = compile_composition_plan(&[], reseal_source(embedding_only)).unwrap();
    assert!(embedding_plan.delegation_graph.edges.is_empty());
    assert_eq!(
        embedding_plan.ui_embedding_graph.edges,
        ["ui-embedding-a", "ui-embedding-b"]
    );
    let encoded = serde_json::to_value(&embedding_plan.declarations.edge_contracts[0]).unwrap();
    for authority_field in [
        "responsibility_contract_id",
        "request_translation_id",
        "result_translation_id",
        "authority_ceiling",
        "budget",
        "provider_binding_id",
    ] {
        assert!(encoded.get(authority_field).is_none());
    }

    let path = serde_json::to_value(&plan.declarations.selected_experience_paths[0]).unwrap();
    assert!(path.get("graph_kind").is_none());
    assert!(path.get("delegation_contract_ids").is_none());
}

#[test]
fn graph_10_rejects_cross_kind_edge_substitution_in_the_work_spine() {
    let mut source = neutral_source();
    let host = ParticipantRefV1::Interface {
        interface_id: "interface-host".into(),
    };
    let embedded = ParticipantRefV1::Interface {
        interface_id: "interface-embedded".into(),
    };
    source.participant_provider_manifests[0]
        .participants
        .extend([host.clone(), embedded.clone()]);
    source.edge_contracts.push(CompositionEdgeV1::UiEmbedding {
        id: "ui-embedding".into(),
        host,
        embedded,
    });
    source.selected_experience_paths[0].work_delegation_edge_ids = vec!["ui-embedding".into()];

    assert!(matches!(
        compile(source),
        Err(CompositionError::Path { .. })
    ));
}

#[test]
fn graph_07_resolves_each_contextual_selection_without_absorbing_terminal_harness() {
    let selections = [
        selection("claude-neutral", "neutral-path"),
        selection("codex-neutral", "neutral-path"),
    ];
    let plan = compile_composition_plan(&selections, reseal_source(neutral_source())).unwrap();
    assert_eq!(plan.reason_paths.len(), 1);

    let encoded = serde_json::to_value(plan).unwrap();
    assert!(encoded.get("execution_paths").is_none());
    assert!(!encoded.to_string().contains("terminal_harness"));

    let missing = [selection("codex-missing", "missing-path")];
    assert!(matches!(
        compile_composition_plan(&missing, reseal_source(neutral_source())),
        Err(CompositionPlanFailure::Declaration(CompositionError::PathSelection {
            selection,
            experience_path,
        })) if selection == "codex-missing" && experience_path == "missing-path"
    ));
}

#[test]
fn graph_07_direct_execution_is_the_zero_delegation_edge_case() {
    let mut source = neutral_source();
    source.edge_contracts.clear();
    source.request_translations.clear();
    source.result_translations.clear();
    source.selected_experience_paths[0]
        .work_delegation_edge_ids
        .clear();
    source.termination_contracts[0].owner = participant("entry");

    let plan = compile(source).expect("direct execution requires no delegation edge");
    assert!(plan.delegation_graph.edges.is_empty());
    assert!(plan.reason_paths[0]
        .reasons
        .iter()
        .all(|reason| !matches!(reason, CompositionReasonStepV1::DelegationEdge { .. })));
}

#[test]
fn graph_08_substitutes_neutral_providers_without_product_branching() {
    let first = compile(neutral_source()).unwrap();
    let mut substituted = neutral_source();
    let providers = &mut substituted.participant_provider_manifests[0].providers;
    providers[0].id = "provider-d".into();
    providers[0].compatible_provider_ids = vec!["provider-c".into()];
    providers[1].id = "provider-c".into();
    providers[1].compatible_provider_ids = vec!["provider-d".into()];
    let second = compile(substituted).unwrap();
    assert_eq!(
        first.declarations.responsibility_contracts,
        second.declarations.responsibility_contracts
    );
    assert_eq!(
        first.declarations.selected_experience_paths,
        second.declarations.selected_experience_paths
    );
    assert_ne!(
        first.declarations.participant_provider_manifests,
        second.declarations.participant_provider_manifests
    );
    let encoded = serde_json::to_string(&second).unwrap();
    for product in ["rawfish", "finance", "comfyui", "website"] {
        assert!(!encoded.to_lowercase().contains(product));
    }
}

#[test]
fn provider_binding_closure_is_order_invariant_and_content_sensitive() {
    let mut plan = compile(neutral_source()).unwrap();
    let participant = plan.declarations.provider_bindings[0].participant.clone();
    let second = provider_binding_with_coordinates(
        participant.clone(),
        "provider-binding-b",
        "executable-a",
    );
    plan.declarations.provider_bindings.push(second);

    let baseline = plan.provider_binding_closure_digest().unwrap();
    plan.declarations.provider_bindings.reverse();
    assert_eq!(
        baseline,
        plan.provider_binding_closure_digest().unwrap(),
        "closure identity must ignore declaration order"
    );

    let binding = plan
        .declarations
        .provider_bindings
        .iter_mut()
        .find(|binding| binding.id == "provider-binding-b")
        .unwrap();
    *binding = provider_binding_with_coordinates(participant, "provider-binding-b", "executable-b");
    assert_ne!(
        baseline,
        plan.provider_binding_closure_digest().unwrap(),
        "closure identity must retain provider-binding content"
    );
}

#[test]
fn graph_09_never_infers_activation_invocation_native_effect_or_occurrence() {
    let plan = compile(neutral_source()).unwrap();
    assert_eq!(plan.plan_digest, plan.recompute_digest().unwrap());
    let value = serde_json::to_value(&plan).unwrap();
    let object = value.as_object().unwrap();
    for absent in [
        "deployment_activation_identity",
        "invocation_identity",
        "native_effect_identity",
        "activation",
        "invocation",
        "native_effect",
        "settlement_occurred",
        "continuity_state",
    ] {
        assert!(!object.contains_key(absent));
    }
}

#[test]
fn fully_populated_sparse_path_round_trips_every_declared_boundary() {
    let source = fully_populated_path_source();
    let plan = compile(source.clone()).unwrap();
    let bytes = serde_json::to_vec(&plan).unwrap();
    let decoded: ExperienceCompositionPlanV1 = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded, plan);
    pale_contracts::validate_record(&decoded, EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID).unwrap();
    assert_eq!(plan.plan_digest, plan.recompute_digest().unwrap());

    let path = &plan.declarations.selected_experience_paths[0];
    assert!(!path.origin_receipt_contract_id.is_empty());
    assert!(!path.entry_contract_id.is_empty());
    assert!(!path.responsibility_contract_ids.is_empty());
    assert!(!path.provider_binding_ids.is_empty());
    assert!(!path.capability_ceiling.is_empty());
    assert!(!path.effect_ceiling.is_empty());
    assert!(!path.invocations.is_empty());
    assert!(!path.native_owner_transitions.is_empty());
    assert!(!path.native_outcomes.is_empty());
    assert!(!path.destination_consequences.is_empty());
    assert!(!path.reconciliation.is_empty());
    assert!(!path
        .compiled_content
        .compiled_content_identity
        .compiled_content_sha256
        .as_str()
        .is_empty());
    assert!(!path
        .compiled_content
        .source_content_identity
        .source_content_sha256
        .as_str()
        .is_empty());
    for coordinate in [
        &path.assurance_subject_id,
        &path.evidence_carrier_id,
        &path.retention_policy_id,
        &path.privacy_policy_id,
        &path.termination_contract_id,
        &path.settlement_contract_id,
        &path.continuity_contract_id,
    ] {
        assert!(!coordinate.is_empty());
    }

    let mut changed = source;
    changed.selected_experience_paths[0].privacy_policy_id = "privacy-policy-b".into();
    assert_ne!(
        compile(changed).unwrap().plan_digest,
        plan.plan_digest,
        "every populated sparse-path field must remain inside the plan identity"
    );
}

#[test]
fn every_identity_axis_is_retained_and_independently_seals_the_plan() {
    let source = fully_populated_path_source();
    let plan = compile(source.clone()).unwrap();
    let expected_axes = selected_identity_axes(&plan);
    let expected_digest = plan.plan_digest.clone();
    let mut mutations = Vec::new();

    let mut changed = source.clone();
    changed.selected_experience_paths[0].privacy_policy_id = "privacy-policy-b".into();
    mutations.push(changed);

    let mut changed = source.clone();
    changed.provider_bindings[0].artifact_coordinates.executable = "executable-b".into();
    mutations.push(changed);

    let mut changed = source.clone();
    changed.provider_bindings[0]
        .physical_deployment
        .physical_deployment_id = "deployment-b".into();
    mutations.push(changed);

    let mut changed = source.clone();
    let changed_invocation = InvocationIdentityV1 {
        invocation_sha256: digest("native-call-a-second-attempt"),
    };
    changed.selected_experience_paths[0].invocations[0].identity = changed_invocation.clone();
    match &mut changed.selected_experience_paths[0].native_outcomes[0] {
        NativeOutcomeV1::ReadOnlyResult {
            invocation_identity,
            ..
        }
        | NativeOutcomeV1::EffectfulOutcome {
            invocation_identity,
            ..
        } => *invocation_identity = changed_invocation,
    }
    mutations.push(changed);

    let mut changed = source.clone();
    let NativeOutcomeV1::EffectfulOutcome {
        native_effect_identity,
        ..
    } = &mut changed.selected_experience_paths[0].native_outcomes[0]
    else {
        panic!("fixture carries one effectful outcome")
    };
    native_effect_identity.native_effect_sha256 = digest("native-effect-b");
    mutations.push(changed);

    let mut changed = source.clone();
    changed.settlement_contracts[0]
        .identity
        .evidence_settlement_sha256 = digest("settlement-b");
    mutations.push(changed);

    let mut changed = source.clone();
    changed.continuity_contracts[0].identity.continuity_sha256 = digest("continuity-b");
    mutations.push(changed);

    let mut changed = source.clone();
    changed.provider_bindings[0]
        .generation_coordinates
        .native_data_generation_identity
        .native_data_generation_sha256 = digest("native-data-generation-b");
    mutations.push(changed);

    let mut changed = source;
    changed.provider_bindings[0]
        .generation_coordinates
        .pale_admission_generation_identity
        .pale_admission_generation_sha256 = digest("pale-admission-generation-b");
    mutations.push(changed);

    for changed in mutations {
        let changed_plan = compile(changed).unwrap();
        let changed_axes = selected_identity_axes(&changed_plan);
        assert_ne!(changed_axes, expected_axes);
        assert_ne!(changed_plan.plan_digest, expected_digest);
    }
}

#[test]
fn stale_binding_path_and_deployment_identity_claims_fail_closed() {
    let mut stale_binding = neutral_source();
    stale_binding.provider_bindings[0]
        .artifact_coordinates
        .executable = "different-executable".into();
    assert!(matches!(
        compile_claims(stale_binding),
        Err(CompositionError::ProviderBinding {
            violation: ProviderBindingViolation::BindingIdentityMismatch,
            ..
        })
    ));

    let mut stale_path = neutral_source();
    stale_path.selected_experience_paths[0].privacy_policy_id = "different-privacy".into();
    assert!(matches!(
        compile_claims(stale_path),
        Err(CompositionError::Path { reason, .. })
            if reason.contains("complete canonical path body")
    ));

    let mut stale_deployment = neutral_source();
    let original_binding_identity = stale_deployment.provider_bindings[0]
        .binding_identity
        .clone();
    stale_deployment.provider_bindings[0]
        .generation_coordinates
        .deployment_activation_identity
        .deployment_activation_sha256 = digest("fabricated-deployment-activation");
    stale_deployment.provider_bindings[0].binding_identity = stale_deployment.provider_bindings[0]
        .recompute_identity()
        .unwrap();
    assert_ne!(
        stale_deployment.provider_bindings[0].binding_identity, original_binding_identity,
        "the complete binding identity must cover its derived deployment identity"
    );
    assert!(matches!(
        compile_claims(stale_deployment),
        Err(CompositionError::ProviderBinding {
            violation: ProviderBindingViolation::DeploymentIdentityMismatch,
            ..
        })
    ));
}

#[test]
fn malformed_identity_and_legacy_relabel_fields_fail_deserialization() {
    macro_rules! rejects_transposed_digest {
        ($identity:ty, $wrong_field:literal) => {{
            let mut object = serde_json::Map::new();
            object.insert(
                $wrong_field.into(),
                serde_json::to_value(digest($wrong_field)).unwrap(),
            );
            assert!(
                serde_json::from_value::<$identity>(serde_json::Value::Object(object)).is_err(),
                "{} accepted transposed field `{}`",
                stringify!($identity),
                $wrong_field,
            );
        }};
    }

    rejects_transposed_digest!(ResponsibilityPathIdentityV1, "provider_binding_sha256");
    rejects_transposed_digest!(ProviderBindingIdentityV1, "deployment_activation_sha256");
    rejects_transposed_digest!(DeploymentActivationIdentityV1, "invocation_sha256");
    rejects_transposed_digest!(InvocationIdentityV1, "native_effect_sha256");
    rejects_transposed_digest!(NativeEffectIdentityV1, "evidence_settlement_sha256");
    rejects_transposed_digest!(EvidenceSettlementIdentityV1, "continuity_sha256");
    rejects_transposed_digest!(ContinuityIdentityV1, "native_data_generation_sha256");
    rejects_transposed_digest!(
        NativeDataGenerationIdentityV1,
        "pale_admission_generation_sha256"
    );
    rejects_transposed_digest!(
        PaleAdmissionGenerationIdentityV1,
        "responsibility_path_sha256"
    );
    let mut stale_path =
        serde_json::to_value(&neutral_source().selected_experience_paths[0]).unwrap();
    stale_path
        .as_object_mut()
        .unwrap()
        .insert("graph_kind".into(), serde_json::json!("ui_embedding"));
    assert!(serde_json::from_value::<ExperiencePathV1>(stale_path).is_err());

    let outcome = NativeOutcomeV1::EffectfulOutcome {
        edge_id: "delegation-entry-worker".into(),
        invocation_identity: InvocationIdentityV1 {
            invocation_sha256: digest("invocation-a"),
        },
        native_effect_identity: NativeEffectIdentityV1 {
            native_effect_sha256: digest("native-effect-a"),
        },
        outcome_contract_id: "native-outcome-contract".into(),
    };
    let mut missing_invocation = serde_json::to_value(&outcome).unwrap();
    missing_invocation
        .as_object_mut()
        .unwrap()
        .remove("invocation_identity");
    assert!(serde_json::from_value::<NativeOutcomeV1>(missing_invocation).is_err());

    let mut transposed_invocation = serde_json::to_value(outcome).unwrap();
    transposed_invocation["invocation_identity"] = serde_json::json!({
        "native_effect_sha256": digest("wrong-domain")
    });
    assert!(serde_json::from_value::<NativeOutcomeV1>(transposed_invocation).is_err());

    let mut read_only = serde_json::to_value(NativeOutcomeV1::ReadOnlyResult {
        edge_id: "delegation-entry-worker".into(),
        invocation_identity: InvocationIdentityV1 {
            invocation_sha256: digest("read-only-invocation"),
        },
        outcome_contract_id: "read-only-result-contract".into(),
    })
    .unwrap();
    read_only["native_effect_identity"] = serde_json::json!({
        "native_effect_sha256": digest("fabricated-effect")
    });
    assert!(serde_json::from_value::<NativeOutcomeV1>(read_only).is_err());
}

#[test]
fn provider_binding_ownership_and_responsibility_rejections_are_distinct() {
    let mut wrong_owner = neutral_source();
    wrong_owner.provider_bindings[0].participant = participant("worker");
    assert!(matches!(
        compile(wrong_owner),
        Err(CompositionError::ProviderBinding {
            violation: ProviderBindingViolation::ParticipantOwnershipMismatch,
            ..
        })
    ));
    let mut wrong_responsibility = neutral_source();
    wrong_responsibility.responsibility_contracts[0].responsibilities =
        vec![ResponsibilityV1::TerminalHarnessExecution];
    assert!(matches!(
        compile(wrong_responsibility),
        Err(CompositionError::ProviderBinding {
            violation: ProviderBindingViolation::ResponsibilityNotGoverned,
            ..
        })
    ));
}

#[test]
fn provider_bindings_require_exact_responsibility_schemas_and_selected_capability_evidence() {
    let mut missing_schema = neutral_source();
    missing_schema.provider_bindings[0]
        .responsibility_schema_identities
        .clear();
    assert!(matches!(
        compile(missing_schema),
        Err(CompositionError::ProviderBinding {
            violation: ProviderBindingViolation::ResponsibilitySchemaSetMismatch,
            ..
        })
    ));

    let mut unrelated_schema = neutral_source();
    unrelated_schema.provider_bindings[0]
        .responsibility_schema_identities
        .push(contract_schema_identity("entry-contract.v1"));
    assert!(matches!(
        compile(unrelated_schema),
        Err(CompositionError::ProviderBinding {
            violation: ProviderBindingViolation::ResponsibilitySchemaSetMismatch,
            ..
        })
    ));

    let extension_schema = contract_schema_identity("capability-procedure.v1");
    let extension = ResponsibilityV1::Extension {
        namespace_id: "example".into(),
        contract_id: "extension-responsibility".into(),
        contract_schema_identity: extension_schema.clone(),
    };
    let mut extension_source = neutral_source();
    extension_source.responsibility_contracts[0].responsibilities = vec![extension.clone()];
    extension_source.provider_bindings[0].responsibility = extension;
    assert!(matches!(
        compile(extension_source.clone()),
        Err(CompositionError::ProviderBinding {
            violation: ProviderBindingViolation::ResponsibilitySchemaSetMismatch,
            ..
        })
    ));
    extension_source.provider_bindings[0]
        .responsibility_schema_identities
        .push(extension_schema.clone());
    compile(extension_source).unwrap();

    let mut wrong_extension_digest = neutral_source();
    let mut wrong_schema = extension_schema;
    wrong_schema.schema_sha256 = SchemaDigest::hash_bytes(b"different schema bytes");
    let extension = ResponsibilityV1::Extension {
        namespace_id: "example".into(),
        contract_id: "extension-responsibility".into(),
        contract_schema_identity: wrong_schema.clone(),
    };
    wrong_extension_digest.responsibility_contracts[0].responsibilities = vec![extension.clone()];
    wrong_extension_digest.provider_bindings[0].responsibility = extension;
    wrong_extension_digest.provider_bindings[0]
        .responsibility_schema_identities
        .push(wrong_schema);
    assert!(matches!(
        compile(wrong_extension_digest),
        Err(CompositionError::DeclaredSchema {
            source: pale_contracts::SchemaError::IdentityMismatch(_),
            ..
        })
    ));

    let mut unrelated_provider = neutral_source();
    unrelated_provider.provider_bindings[0]
        .conformance_coordinates
        .per_capability_evidence[0]
        .capability_id = "other-capability".into();
    let mut covering_provider = unrelated_provider.provider_bindings[0].clone();
    covering_provider.id = "provider-binding-b".into();
    covering_provider.binding_identity = ProviderBindingIdentityV1 {
        provider_binding_sha256: digest("provider-binding-b"),
    };
    covering_provider
        .conformance_coordinates
        .per_capability_evidence[0]
        .capability_id = "neutral-capability".into();
    unrelated_provider.provider_bindings.push(covering_provider);
    assert!(matches!(
        compile(unrelated_provider.clone()),
        Err(CompositionError::Path { .. })
    ));
    unrelated_provider.selected_experience_paths[0].provider_binding_ids =
        vec!["provider-binding-b".into()];
    compile(unrelated_provider).unwrap();
}

#[test]
fn selected_provider_evidence_cannot_enter_from_an_off_path_participant() {
    let mut source = neutral_source();
    source.provider_bindings[0]
        .conformance_coordinates
        .per_capability_evidence[0]
        .capability_id = "other-capability".into();

    let off_path_participant = engine("provider-a-participant");
    source
        .responsibility_contracts
        .push(ResponsibilityContractV1 {
            id: "responsibility-off-path".into(),
            participant: off_path_participant.clone(),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        });
    let mut off_path_binding = provider_binding(off_path_participant);
    off_path_binding.id = "provider-binding-off-path".into();
    off_path_binding.binding_identity = ProviderBindingIdentityV1 {
        provider_binding_sha256: digest("provider-binding-off-path"),
    };
    off_path_binding.responsibility_contract_ids = vec!["responsibility-off-path".into()];
    source.provider_bindings.push(off_path_binding);
    source.selected_experience_paths[0]
        .provider_binding_ids
        .push("provider-binding-off-path".into());
    source.selected_experience_paths[0]
        .responsibility_contract_ids
        .push("responsibility-off-path".into());

    assert!(matches!(
        compile(source),
        Err(CompositionError::Path { .. })
    ));
}

#[test]
fn runtime_effect_cycles_cannot_hide_across_edge_kinds() {
    let mut source = neutral_source();
    let application = ParticipantRefV1::Application {
        application_id: "cycle-application".into(),
    };
    let worker = participant("cycle-worker");
    source.participant_provider_manifests[0]
        .participants
        .extend([application.clone(), worker.clone()]);
    source.responsibility_contracts.extend([
        ResponsibilityContractV1 {
            id: "responsibility-cycle-application".into(),
            participant: application.clone(),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        },
        ResponsibilityContractV1 {
            id: "responsibility-cycle-worker".into(),
            participant: worker.clone(),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        },
    ]);
    let mut worker_binding = provider_binding(worker.clone());
    worker_binding.id = "provider-binding-cycle-worker".into();
    worker_binding.binding_identity = ProviderBindingIdentityV1 {
        provider_binding_sha256: digest("provider-binding-cycle-worker"),
    };
    worker_binding.responsibility_contract_ids = vec!["responsibility-cycle-worker".into()];
    source.provider_bindings.push(worker_binding);
    source.request_translations.extend([
        RequestTranslationV1 {
            id: "request-cycle-work".into(),
            from: application.clone(),
            to: worker.clone(),
            input_contract: "cycle-work-input".into(),
            output_contract: "cycle-worker-input".into(),
            preserves_authority: true,
        },
        RequestTranslationV1 {
            id: "request-cycle-native".into(),
            from: worker.clone(),
            to: application.clone(),
            input_contract: "cycle-native-input".into(),
            output_contract: "cycle-application-input".into(),
            preserves_authority: true,
        },
    ]);
    source.result_translations.extend([
        ResultTranslationV1 {
            id: "result-cycle-work".into(),
            from: worker.clone(),
            to: application.clone(),
            input_contract: "cycle-worker-result".into(),
            output_contract: "cycle-work-result".into(),
            preserves_evidence: true,
        },
        ResultTranslationV1 {
            id: "result-cycle-native".into(),
            from: application.clone(),
            to: worker.clone(),
            input_contract: "cycle-application-result".into(),
            output_contract: "cycle-native-result".into(),
            preserves_evidence: true,
        },
    ]);
    source.edge_contracts.extend([
        CompositionEdgeV1::WorkDelegation {
            id: "cycle-work".into(),
            from: application.clone(),
            to: worker.clone(),
            responsibility_contract_id: "responsibility-cycle-application".into(),
            request_translation_id: "request-cycle-work".into(),
            result_translation_id: "result-cycle-work".into(),
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
            budget: budget(),
        },
        CompositionEdgeV1::NativeApplicationCall {
            id: "cycle-native".into(),
            caller: worker,
            application,
            provider_binding_id: "provider-binding-cycle-worker".into(),
            request_translation_id: "request-cycle-native".into(),
            result_translation_id: "result-cycle-native".into(),
        },
    ]);

    assert!(matches!(
        compile(source),
        Err(CompositionError::Path { .. })
    ));
}

#[test]
fn ui_embedding_requires_interface_containment_at_both_endpoints() {
    let mut source = neutral_source();
    source.edge_contracts.push(CompositionEdgeV1::UiEmbedding {
        id: "invalid-ui-embedding".into(),
        host: participant("entry"),
        embedded: engine("provider-a-participant"),
    });

    assert!(matches!(
        compile(source),
        Err(CompositionError::Edge { .. })
    ));
}

#[test]
fn selected_path_ceilings_intersect_and_may_only_narrow_each_edge() {
    let mut disjoint = neutral_source();
    disjoint
        .responsibility_contracts
        .push(ResponsibilityContractV1 {
            id: "responsibility-entry-disjoint".into(),
            participant: participant("entry"),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Publish],
        });
    disjoint.provider_bindings[0]
        .responsibility_contract_ids
        .push("responsibility-entry-disjoint".into());
    disjoint.selected_experience_paths[0]
        .responsibility_contract_ids
        .push("responsibility-entry-disjoint".into());
    assert!(matches!(
        compile(disjoint),
        Err(CompositionError::Path { reason, .. })
            if reason == "path effect ceiling exceeds its selected responsibility contracts"
    ));

    let mut unused = neutral_source();
    unused
        .responsibility_contracts
        .push(ResponsibilityContractV1 {
            id: "responsibility-unused".into(),
            participant: participant("worker"),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        });
    unused.selected_experience_paths[0]
        .responsibility_contract_ids
        .push("responsibility-unused".into());
    assert!(matches!(
        compile(unused),
        Err(CompositionError::Path { reason, .. })
            if reason == "selected responsibility contracts do not exactly match the connected path closure"
    ));

    let mut narrowed = neutral_source();
    narrowed.selected_experience_paths[0].effect_ceiling.clear();
    compile(narrowed).unwrap();

    let mut edge_widening = neutral_source();
    edge_widening.responsibility_contracts[0]
        .authority_ceiling
        .push(pale_core::records::EffectClass::Write);
    edge_widening.selected_experience_paths[0].effect_ceiling =
        vec![pale_core::records::EffectClass::Write];
    assert!(matches!(
        compile(edge_widening),
        Err(CompositionError::Path { .. })
    ));
}

#[test]
fn invocation_order_is_a_connected_causal_traversal() {
    compile(native_invocation_chain_source()).unwrap();

    let mut reversed = native_invocation_chain_source();
    reversed.selected_experience_paths[0].invocations.reverse();
    assert!(matches!(
        compile(reversed),
        Err(CompositionError::Path { reason, .. })
            if reason.contains("disconnected from the selected path")
    ));

    let mut disconnected = native_invocation_chain_source();
    disconnected.selected_experience_paths[0]
        .invocations
        .remove(0);
    assert!(matches!(
        compile(disconnected),
        Err(CompositionError::Path { reason, .. })
            if reason.contains("disconnected from the selected path")
    ));
}

#[test]
fn composition_identities_are_unique_and_outcomes_join_exact_invocations() {
    let mut duplicate_binding = neutral_source();
    let mut second_binding = duplicate_binding.provider_bindings[0].clone();
    second_binding.id = "provider-binding-b".into();
    duplicate_binding.provider_bindings.push(second_binding);
    assert!(matches!(
        compile_claims(duplicate_binding),
        Err(CompositionError::DuplicateId {
            kind: "provider binding identity",
            ..
        })
    ));

    let mut duplicate_path = neutral_source();
    let mut second_path = duplicate_path.selected_experience_paths[0].clone();
    second_path.id = "neutral-path-b".into();
    duplicate_path.selected_experience_paths.push(second_path);
    assert!(matches!(
        compile_claims(duplicate_path),
        Err(CompositionError::DuplicateId {
            kind: "responsibility path identity",
            ..
        })
    ));

    let invocation_a = InvocationIdentityV1 {
        invocation_sha256: digest("invocation-a"),
    };
    let invocation_b = InvocationIdentityV1 {
        invocation_sha256: digest("invocation-b"),
    };
    let mut repeated_edge = neutral_source();
    repeated_edge.selected_experience_paths[0].invocations = vec![
        PathInvocationV1 {
            edge_id: "delegation-entry-worker".into(),
            identity: invocation_a.clone(),
        },
        PathInvocationV1 {
            edge_id: "delegation-entry-worker".into(),
            identity: invocation_b.clone(),
        },
    ];
    repeated_edge.selected_experience_paths[0].native_outcomes =
        vec![NativeOutcomeV1::EffectfulOutcome {
            edge_id: "delegation-entry-worker".into(),
            invocation_identity: invocation_a.clone(),
            native_effect_identity: NativeEffectIdentityV1 {
                native_effect_sha256: digest("native-effect-a"),
            },
            outcome_contract_id: "native-outcome-contract".into(),
        }];
    compile(repeated_edge.clone()).unwrap();

    let mut duplicate_invocation = repeated_edge.clone();
    duplicate_invocation.selected_experience_paths[0].invocations[1].identity = invocation_a;
    assert!(matches!(
        compile(duplicate_invocation),
        Err(CompositionError::DuplicateId {
            kind: "invocation identity",
            ..
        })
    ));

    let mut duplicate_effect = repeated_edge.clone();
    let mut second_outcome =
        duplicate_effect.selected_experience_paths[0].native_outcomes[0].clone();
    match &mut second_outcome {
        NativeOutcomeV1::ReadOnlyResult {
            invocation_identity,
            outcome_contract_id,
            ..
        }
        | NativeOutcomeV1::EffectfulOutcome {
            invocation_identity,
            outcome_contract_id,
            ..
        } => {
            *invocation_identity = invocation_b;
            *outcome_contract_id = "second-outcome-contract".into();
        }
    }
    duplicate_effect.selected_experience_paths[0]
        .native_outcomes
        .push(second_outcome);
    assert!(matches!(
        compile(duplicate_effect),
        Err(CompositionError::DuplicateId {
            kind: "native effect identity",
            ..
        })
    ));

    let mut unknown_invocation = repeated_edge;
    match &mut unknown_invocation.selected_experience_paths[0].native_outcomes[0] {
        NativeOutcomeV1::ReadOnlyResult {
            invocation_identity,
            ..
        }
        | NativeOutcomeV1::EffectfulOutcome {
            invocation_identity,
            ..
        } => {
            *invocation_identity = InvocationIdentityV1 {
                invocation_sha256: digest("unknown-invocation"),
            };
        }
    }
    assert!(matches!(
        compile(unknown_invocation),
        Err(CompositionError::Path { reason, .. })
            if reason == "native outcome references edge `delegation-entry-worker` without its exact invocation identity"
    ));

    let terminal_leaf = ParticipantRefV1::TerminalLeaf {
        terminal_leaf_id: "terminal-leaf".into(),
    };
    let mut wrong_edge = neutral_source();
    wrong_edge.participant_provider_manifests[0]
        .participants
        .push(terminal_leaf.clone());
    wrong_edge.termination_contracts[0].terminal_leaf = Some(terminal_leaf.clone());
    wrong_edge
        .edge_contracts
        .push(CompositionEdgeV1::TerminalLeaf {
            id: "terminal-edge".into(),
            owner: participant("worker"),
            terminal_leaf,
            termination_contract_id: "termination-worker".into(),
        });
    wrong_edge.selected_experience_paths[0].invocations = vec![
        PathInvocationV1 {
            edge_id: "delegation-entry-worker".into(),
            identity: InvocationIdentityV1 {
                invocation_sha256: digest("work-invocation"),
            },
        },
        PathInvocationV1 {
            edge_id: "terminal-edge".into(),
            identity: InvocationIdentityV1 {
                invocation_sha256: digest("terminal-invocation"),
            },
        },
    ];
    wrong_edge.selected_experience_paths[0].native_outcomes =
        vec![NativeOutcomeV1::EffectfulOutcome {
            edge_id: "delegation-entry-worker".into(),
            invocation_identity: InvocationIdentityV1 {
                invocation_sha256: digest("terminal-invocation"),
            },
            native_effect_identity: NativeEffectIdentityV1 {
                native_effect_sha256: digest("wrong-edge-effect"),
            },
            outcome_contract_id: "wrong-edge-outcome".into(),
        }];
    assert!(matches!(
        compile(wrong_edge),
        Err(CompositionError::Path { reason, .. })
            if reason == "native outcome references edge `delegation-entry-worker` without its exact invocation identity"
    ));

    let mut duplicate_settlement = neutral_source();
    let mut second_settlement = duplicate_settlement.settlement_contracts[0].clone();
    second_settlement.id = "settlement-worker-b".into();
    duplicate_settlement
        .settlement_contracts
        .push(second_settlement);
    assert!(matches!(
        compile(duplicate_settlement),
        Err(CompositionError::DuplicateId {
            kind: "evidence settlement identity",
            ..
        })
    ));

    let mut duplicate_continuity = neutral_source();
    let mut second_continuity = duplicate_continuity.continuity_contracts[0].clone();
    second_continuity.id = "continuity-entry-b".into();
    duplicate_continuity
        .continuity_contracts
        .push(second_continuity);
    assert!(matches!(
        compile(duplicate_continuity),
        Err(CompositionError::DuplicateId {
            kind: "continuity identity",
            ..
        })
    ));
}

#[test]
fn sparse_path_rejects_ambiguous_transitions_destinations_and_frontiers() {
    let mut duplicate_transition = neutral_source();
    duplicate_transition.selected_experience_paths[0].native_owner_transitions = vec![
        NativeOwnerTransitionV1 {
            edge_id: "delegation-entry-worker".into(),
            native_owner_id: "native-owner".into(),
            transition_contract_id: "transition-a".into(),
            receipt_contract_id: "receipt-a".into(),
        },
        NativeOwnerTransitionV1 {
            edge_id: "delegation-entry-worker".into(),
            native_owner_id: "native-owner".into(),
            transition_contract_id: "transition-b".into(),
            receipt_contract_id: "receipt-b".into(),
        },
    ];
    assert!(matches!(
        compile(duplicate_transition),
        Err(CompositionError::Path { .. })
    ));

    let selected_destination = DestinationConsequenceV1 {
        destination: participant("worker"),
        consequence_contract_id: "consequence".into(),
    };
    let mut valid_destination = neutral_source();
    valid_destination.selected_experience_paths[0].destination_consequences =
        vec![selected_destination.clone()];
    compile(valid_destination).unwrap();

    let mut duplicate_destination = neutral_source();
    duplicate_destination.selected_experience_paths[0].destination_consequences =
        vec![selected_destination.clone(), selected_destination];
    assert!(matches!(
        compile(duplicate_destination),
        Err(CompositionError::Path { .. })
    ));

    let mut off_path_destination = neutral_source();
    off_path_destination.selected_experience_paths[0].destination_consequences =
        vec![DestinationConsequenceV1 {
            destination: engine("provider-a-participant"),
            consequence_contract_id: "consequence".into(),
        }];
    assert!(matches!(
        compile(off_path_destination),
        Err(CompositionError::Path { .. })
    ));

    let mut undeclared_destination = neutral_source();
    undeclared_destination.selected_experience_paths[0].destination_consequences =
        vec![DestinationConsequenceV1 {
            destination: participant("undeclared"),
            consequence_contract_id: "consequence".into(),
        }];
    assert!(matches!(
        compile(undeclared_destination),
        Err(CompositionError::MissingReference { .. })
    ));

    let requirement = ReconciliationRequirementV1 {
        obligation_contract_id: "reconcile".into(),
        frontier_id: "frontier".into(),
    };
    let mut duplicate_reconciliation = neutral_source();
    duplicate_reconciliation.selected_experience_paths[0].reconciliation =
        vec![requirement.clone(), requirement];
    assert!(matches!(
        compile(duplicate_reconciliation),
        Err(CompositionError::Path { .. })
    ));
}

#[test]
fn embedding_and_plugin_dependencies_cannot_regain_invocation_authority() {
    let host = ParticipantRefV1::Interface {
        interface_id: "host".into(),
    };
    let embedded = ParticipantRefV1::Interface {
        interface_id: "embedded".into(),
    };
    let mut embedding = neutral_source();
    embedding.participant_provider_manifests[0]
        .participants
        .extend([host.clone(), embedded.clone()]);
    embedding
        .edge_contracts
        .push(CompositionEdgeV1::UiEmbedding {
            id: "embedding".into(),
            host,
            embedded,
        });
    embedding.selected_experience_paths[0]
        .invocations
        .push(PathInvocationV1 {
            edge_id: "embedding".into(),
            identity: InvocationIdentityV1 {
                invocation_sha256: digest("embedding-invocation"),
            },
        });
    assert!(matches!(
        compile(embedding),
        Err(CompositionError::Path { .. })
    ));

    let plugin = ParticipantRefV1::Plugin {
        plugin_id: "plugin".into(),
    };
    let dependency = ParticipantRefV1::Plugin {
        plugin_id: "dependency".into(),
    };
    let mut plugin_dependency = neutral_source();
    plugin_dependency.participant_provider_manifests[0]
        .participants
        .extend([plugin.clone(), dependency.clone()]);
    plugin_dependency
        .edge_contracts
        .push(CompositionEdgeV1::PluginDependency {
            id: "plugin-dependency".into(),
            plugin,
            dependency,
        });
    plugin_dependency.selected_experience_paths[0]
        .invocations
        .push(PathInvocationV1 {
            edge_id: "plugin-dependency".into(),
            identity: InvocationIdentityV1 {
                invocation_sha256: digest("plugin-invocation"),
            },
        });
    assert!(matches!(
        compile(plugin_dependency),
        Err(CompositionError::Path { .. })
    ));
}

#[test]
fn missing_translation_and_authority_escalation_fail_closed() {
    let mut missing = neutral_source();
    missing.result_translations.clear();
    assert!(matches!(
        compile(missing),
        Err(CompositionError::MissingReference { .. })
    ));

    let mut escalation = neutral_source();
    let CompositionEdgeV1::WorkDelegation {
        authority_ceiling, ..
    } = &mut escalation.edge_contracts[0]
    else {
        panic!("fixture edge must remain a work delegation");
    };
    *authority_ceiling = vec![pale_core::records::EffectClass::Publish];
    assert!(matches!(
        compile(escalation),
        Err(CompositionError::Delegation { .. })
    ));
}
