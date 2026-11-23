//! Shared, source-shaped neutral composition fixtures.

use pale_core::records::{
    CommunicationActorHandleV1, CompiledContentCoordinateV1, CompiledContentIdentityV1,
    CompositionEdgeV1, ContinuityContractV1, ContinuityIdentityV1, EntryContractV1,
    EntryOriginRefV1, EvidenceSettlementIdentityV1, ExperienceCompositionDeclarationsV1,
    ExperiencePathPreimageV1, ExperiencePathV1, InvocationIdentityV1,
    ParticipantProviderManifestV1, ParticipantRefV1, PathBudgetV1, PathInvocationV1,
    ProviderBindingV1, ProviderCompatibilityDeclarationV1, RequestTranslationV1,
    ResponsibilityContractV1, ResponsibilityV1, ResultTranslationV1, SchemaId,
    SettlementContractV1, SettlementDispositionV1, SourceContentIdentityV1, TerminationContractV1,
    TerminationKindV1, WorkspaceCompositionSourceV1, WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA,
    WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_VERSION,
};

use super::provider_binding_fixtures::{digest, provider_binding};

pub(crate) fn participant(id: &str) -> ParticipantRefV1 {
    ParticipantRefV1::Capability {
        capability_id: id.into(),
    }
}

pub(crate) fn engine(id: &str) -> ParticipantRefV1 {
    ParticipantRefV1::Engine {
        engine_id: id.into(),
    }
}

pub(crate) fn budget() -> PathBudgetV1 {
    PathBudgetV1 {
        max_depth: 8,
        max_fan_out: 4,
        max_time_ms: 100,
        max_cost_units: 100,
        max_resource_units: 100,
        max_effect_count: 8,
        max_recursion_iterations: 8,
    }
}

/// Build one source document whose selected paths differ only by exact identity.
///
/// Host-projection tests use this owner once rather than copying the populated
/// composition graph or weakening path selection with empty declarations.
pub(crate) fn workspace_composition_source(
    selected_path_ids: &[&str],
) -> WorkspaceCompositionSourceV1 {
    assert!(
        !selected_path_ids.is_empty(),
        "a composition fixture must select at least one experience path"
    );
    let mut declarations = populated_composition_declarations();
    let path_template = declarations.selected_experience_paths[0].to_preimage();
    let binding_template = declarations.provider_bindings[0].to_preimage();
    declarations.provider_bindings = selected_path_ids
        .iter()
        .map(|id| {
            let mut binding = binding_template.clone();
            let is_codex = id.contains("codex");
            binding.id = if is_codex {
                "provider-binding-codex"
            } else {
                "provider-binding-claude-code"
            }
            .into();
            binding.adapter_plugin.adapter_plugin_id = if is_codex {
                "pale.adapter.codex.v1"
            } else {
                "pale.adapter.claude-code.v1"
            }
            .into();
            ProviderBindingV1::seal(binding).expect("fixture provider binding seals")
        })
        .collect();
    declarations
        .provider_bindings
        .sort_by(|left, right| left.id.cmp(&right.id));
    declarations
        .provider_bindings
        .dedup_by(|left, right| left.id == right.id);
    declarations.selected_experience_paths = selected_path_ids
        .iter()
        .map(|id| {
            let mut path = path_template.clone();
            let provider_binding_id = if id.contains("codex") {
                "provider-binding-codex"
            } else {
                "provider-binding-claude-code"
            };
            path.id = (*id).into();
            path.provider_binding_ids = vec![provider_binding_id.into()];
            for invocation in &mut path.invocations {
                invocation.identity.invocation_sha256 =
                    digest(&format!("{}:{id}", invocation.edge_id));
            }
            ExperiencePathV1::seal(path).expect("fixture experience path seals")
        })
        .collect();
    WorkspaceCompositionSourceV1 {
        schema: SchemaId::parse(WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA)
            .expect("workspace composition schema identity is valid"),
        schema_version: WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_VERSION,
        declarations,
    }
}

pub(crate) fn neutral_composition_declarations() -> ExperienceCompositionDeclarationsV1 {
    let entry = participant("entry");
    let worker = participant("worker");
    let provider_a = engine("provider-a-participant");
    let provider_b = engine("provider-b-participant");
    ExperienceCompositionDeclarationsV1 {
        responsibility_contracts: vec![ResponsibilityContractV1 {
            id: "responsibility-entry".into(),
            participant: entry.clone(),
            responsibilities: vec![ResponsibilityV1::OrganizationalWork],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        }],
        participant_provider_manifests: vec![ParticipantProviderManifestV1 {
            id: "neutral-participants".into(),
            participants: vec![
                worker.clone(),
                provider_b.clone(),
                entry.clone(),
                provider_a.clone(),
            ],
            providers: vec![
                ProviderCompatibilityDeclarationV1 {
                    id: "provider-b".into(),
                    participant: provider_b,
                    capability_ids: vec!["neutral-capability".into()],
                    compatible_provider_ids: vec!["provider-a".into()],
                },
                ProviderCompatibilityDeclarationV1 {
                    id: "provider-a".into(),
                    participant: provider_a,
                    capability_ids: vec!["neutral-capability".into()],
                    compatible_provider_ids: vec!["provider-b".into()],
                },
            ],
        }],
        provider_bindings: vec![provider_binding(entry.clone())],
        entry_contracts: vec![EntryContractV1 {
            id: "entry-contract".into(),
            entry: entry.clone(),
            origin: EntryOriginRefV1::User {
                user_entry_id: "explicit-user-entry".into(),
            },
            responsibility_contract_id: "responsibility-entry".into(),
        }],
        request_translations: vec![RequestTranslationV1 {
            id: "request-entry-worker".into(),
            from: entry.clone(),
            to: worker.clone(),
            input_contract: "neutral-input".into(),
            output_contract: "worker-input".into(),
            preserves_authority: true,
        }],
        result_translations: vec![ResultTranslationV1 {
            id: "result-worker-entry".into(),
            from: worker.clone(),
            to: entry.clone(),
            input_contract: "worker-result".into(),
            output_contract: "neutral-result".into(),
            preserves_evidence: true,
        }],
        termination_contracts: vec![TerminationContractV1 {
            id: "termination-worker".into(),
            owner: worker.clone(),
            kinds: vec![TerminationKindV1::Failed, TerminationKindV1::Completed],
            terminal_leaf: None,
        }],
        settlement_contracts: vec![SettlementContractV1 {
            id: "settlement-worker".into(),
            observer: worker,
            disposition: SettlementDispositionV1::ReadBackRequired,
            native_receipt_contract: None,
            identity: EvidenceSettlementIdentityV1 {
                evidence_settlement_sha256: digest("settlement-worker"),
            },
        }],
        continuity_contracts: vec![ContinuityContractV1 {
            id: "continuity-entry".into(),
            owner: entry.clone(),
            obligations: vec!["retain only declared continuity references".into()],
            identity: ContinuityIdentityV1 {
                continuity_sha256: digest("continuity-entry"),
            },
        }],
        edge_contracts: vec![CompositionEdgeV1::WorkDelegation {
            id: "delegation-entry-worker".into(),
            from: entry,
            to: participant("worker"),
            responsibility_contract_id: "responsibility-entry".into(),
            request_translation_id: "request-entry-worker".into(),
            result_translation_id: "result-worker-entry".into(),
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
            budget: PathBudgetV1 {
                max_time_ms: 10,
                max_cost_units: 10,
                max_resource_units: 10,
                max_effect_count: 1,
                ..budget()
            },
        }],
        selected_experience_paths: vec![ExperiencePathV1::seal(ExperiencePathPreimageV1 {
            id: "neutral-path".into(),
            origin_receipt_contract_id: "origin-receipt".into(),
            entry_contract_id: "entry-contract".into(),
            work_delegation_edge_ids: vec!["delegation-entry-worker".into()],
            responsibility_contract_ids: vec!["responsibility-entry".into()],
            provider_binding_ids: vec!["provider-binding-a".into()],
            capability_ceiling: vec!["neutral-capability".into()],
            effect_ceiling: vec![pale_core::records::EffectClass::Read],
            invocations: Vec::new(),
            native_owner_transitions: Vec::new(),
            native_outcomes: Vec::new(),
            destination_consequences: Vec::new(),
            reconciliation: Vec::new(),
            compiled_content: CompiledContentCoordinateV1 {
                compiled_content_identity: CompiledContentIdentityV1 {
                    compiled_content_sha256: digest("compiled-content"),
                },
                source_content_identity: SourceContentIdentityV1 {
                    source_content_sha256: digest("neutral-source"),
                },
            },
            assurance_subject_id: "assurance-subject".into(),
            evidence_carrier_id: "evidence-carrier".into(),
            retention_policy_id: "retention-policy".into(),
            privacy_policy_id: "privacy-policy".into(),
            termination_contract_id: "termination-worker".into(),
            settlement_contract_id: "settlement-worker".into(),
            continuity_contract_id: "continuity-entry".into(),
            budget: budget(),
        })
        .expect("shared experience path seals")],
    }
}

/// Extend the neutral fixture with every non-path operational/view edge family.
pub(crate) fn populated_composition_declarations() -> ExperienceCompositionDeclarationsV1 {
    let mut declarations = neutral_composition_declarations();
    let entry = participant("entry");
    let worker = participant("worker");
    let terminal = ParticipantRefV1::TerminalLeaf {
        terminal_leaf_id: "terminal-output".into(),
    };
    let interface_host = ParticipantRefV1::Interface {
        interface_id: "interface-host".into(),
    };
    let interface_embedded = ParticipantRefV1::Interface {
        interface_id: "interface-embedded".into(),
    };
    let plugin = ParticipantRefV1::Plugin {
        plugin_id: "plugin-owner".into(),
    };
    let plugin_dependency = ParticipantRefV1::Plugin {
        plugin_id: "plugin-dependency".into(),
    };
    declarations.participant_provider_manifests[0]
        .participants
        .extend([
            terminal.clone(),
            interface_host.clone(),
            interface_embedded.clone(),
            plugin.clone(),
            plugin_dependency.clone(),
        ]);
    declarations
        .responsibility_contracts
        .push(ResponsibilityContractV1 {
            id: "responsibility-terminal".into(),
            participant: terminal.clone(),
            responsibilities: vec![ResponsibilityV1::TerminalHarnessExecution],
            authority_ceiling: vec![pale_core::records::EffectClass::Read],
        });
    let mut terminal_binding = declarations.provider_bindings[0].to_preimage();
    terminal_binding.participant = terminal.clone();
    terminal_binding.responsibility = ResponsibilityV1::TerminalHarnessExecution;
    terminal_binding.responsibility_contract_ids = vec!["responsibility-terminal".into()];
    terminal_binding.adapter_plugin.adapter_plugin_id = "pale.adapter.claude-code.v1".into();
    terminal_binding.communication_actor_handle = Some(CommunicationActorHandleV1 {
        namespace_id: "provider-a".into(),
        canonical_opaque_handle: "actor-entry".into(),
        equality_scope: "synthetic-fixture".into(),
        display_form: Some("Entry actor".into()),
        native_authorship_verifier_id: "verify-actor".into(),
        reference_mapping_id: "map-actor-reference".into(),
        lifecycle_evidence_contract_id: "actor-lifecycle".into(),
        succession_evidence_contract_id: "actor-succession".into(),
    });
    declarations.provider_bindings[0] =
        ProviderBindingV1::seal(terminal_binding).expect("terminal provider binding seals");
    declarations.settlement_contracts[0].native_receipt_contract = Some("native-receipt".into());
    declarations.termination_contracts[0].terminal_leaf = Some(terminal.clone());
    declarations.selected_experience_paths[0].responsibility_contract_ids = vec![
        "responsibility-entry".into(),
        "responsibility-terminal".into(),
    ];
    declarations.selected_experience_paths[0].invocations = vec![PathInvocationV1 {
        edge_id: "terminal-worker-output".into(),
        identity: InvocationIdentityV1 {
            invocation_sha256: digest("terminal-worker-output"),
        },
    }];
    declarations.selected_experience_paths[0].identity = declarations.selected_experience_paths[0]
        .recompute_identity()
        .expect("populated path identity");
    declarations.edge_contracts.extend([
        CompositionEdgeV1::TerminalLeaf {
            id: "terminal-worker-output".into(),
            owner: worker.clone(),
            terminal_leaf: terminal,
            termination_contract_id: "termination-worker".into(),
        },
        CompositionEdgeV1::ObservationSettlement {
            id: "observe-entry".into(),
            subject: entry.clone(),
            observer: worker.clone(),
            settlement_contract_id: "settlement-worker".into(),
        },
        CompositionEdgeV1::ObservationSettlement {
            id: "observe-entry-again".into(),
            subject: entry.clone(),
            observer: worker.clone(),
            settlement_contract_id: "settlement-worker".into(),
        },
        CompositionEdgeV1::UiEmbedding {
            id: "ui-entry-worker".into(),
            host: interface_host,
            embedded: interface_embedded,
        },
        CompositionEdgeV1::PluginDependency {
            id: "plugin-worker-entry".into(),
            plugin,
            dependency: plugin_dependency,
        },
    ]);
    declarations
}
