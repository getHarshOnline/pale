//! Canonical fixtures that keep operation-admission cases on one sealed contract model.

use pale_application::hooks::{parse_stdin_event, HookEvent, StdinEvent, StdinEventFields};
use pale_application::operation_admission::{
    AdmittedOperation, AdmittedToolCall, CompletedToolCall, ObservedToolInvocation,
    OperationAdmissionError, RecordedToolFailure, ToolInvocation, VerifiedOperationBundle,
};
use pale_application::session_state::{ActiveOperationLease, SessionRuntimeState};
use pale_application::terminal_adapters::{for_exact_binding, TerminalAdapterDescriptor};
use pale_core::records::{
    Approval, ArtifactDigest, CapabilityIdV1, CapabilityProvider, CapabilityRequirementIdV1,
    EffectClass, ExecutionPathSelectionIdV1, ExecutionPathSelectionV1, ExecutionPathWitnessV1,
    ExperienceCompositionPlanDigest, FailurePosture, HostToolBinding, HostedProviderExecutionV1,
    InstanceDigest, LogicalRepositoryIdV1, NativeRustProviderExecutionV1, NonRootJsonPointer,
    OperationCapabilityContractV1, OperationCapabilityProviderStateV1, OperationCatalogEntryV1,
    OperationContractCatalogPreimageV1, OperationContractCatalogV1, OperationContractMechanicsV1,
    OperationContractPreimageV1, OperationContractV1, OperationDispatchV1,
    OperationRouteBindingPreimageV1, OperationRouteBindingV1, OperationSemanticDependencyV1,
    PackageIdV1, PackageResourceIdV1, PackageResourceRefV1, ParticipantRefV1, ProfileDigest,
    ProviderBindingRefV1, ResolvedCapabilityFallbackV1, ResolvedCapabilityRequirementPolicyV1,
    ResponsibilityPathIdentityV1, ResponsibilityV1, RouteCapabilityProviderV1, SealedArtifactRef,
    SemanticDependencyKind, ToolObservability, TurnIdentityDigest,
};

/// Exact observable host identity used by every native-action admission case.
pub(super) const NATIVE_DIGEST_TOOL: &str = "mcp__pale__compute_path_inventory";
/// Stable operation-body bytes whose digest is bound into the fixture catalog.
pub(super) const SKILL_BYTES: &[u8] = b"sealed release procedure";
/// Stable invocation-policy bytes whose digest is independently verified.
pub(super) const POLICY_BYTES: &[u8] = b"sealed invocation policy";
/// Stable semantic-dependency bytes used to reject cross-artifact substitution.
pub(super) const DEPENDENCY_BYTES: &[u8] = b"sealed reviewer doctrine";

/// Stable machine-instance identity shared by operation-admission cases.
pub(super) fn instance_digest() -> &'static InstanceDigest {
    // Parsed once so every fixture borrows one exact typed instance identity.
    static DIGEST: std::sync::OnceLock<InstanceDigest> = std::sync::OnceLock::new();
    DIGEST.get_or_init(|| InstanceDigest::parse("b".repeat(64)).unwrap())
}

/// Build one typed Package resource identity used by this fixture family.
pub(super) fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("test").expect("test Package identity is valid"),
        resource_id: PackageResourceIdV1::parse(id).expect("test resource identity is valid"),
    }
}

/// Build one typed operation-local capability requirement identity.
pub(super) fn requirement(id: &str) -> CapabilityRequirementIdV1 {
    CapabilityRequirementIdV1::parse(id).expect("test requirement identity is valid")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TestAdapter {
    ClaudeCode,
    Codex,
}

impl TestAdapter {
    fn binding(self) -> pale_core::records::ProviderBindingV1 {
        let mut binding =
            crate::provider_binding_fixtures::provider_binding(ParticipantRefV1::TerminalLeaf {
                terminal_leaf_id: "terminal-owner".into(),
            });
        binding.responsibility = ResponsibilityV1::TerminalHarnessExecution;
        binding.adapter_plugin.adapter_plugin_id = match self {
            Self::ClaudeCode => "pale.adapter.claude-code.v1",
            Self::Codex => "pale.adapter.codex.v1",
        }
        .into();
        binding.binding_identity = binding.recompute_identity().unwrap();
        binding
    }
}

/// Complete sealed direct-operation fixture with mutable contract variants.
pub(super) struct Fixture {
    pub(super) adapter: TerminalAdapterDescriptor,
    pub(super) adapter_kind: TestAdapter,
    pub(super) provider_binding: ProviderBindingRefV1,
    pub(super) execution_path_witness: ExecutionPathWitnessV1,
    pub(super) profile: ProfileDigest,
    pub(super) repository: LogicalRepositoryIdV1,
    pub(super) binding_reference: SealedArtifactRef,
    pub(super) binding_bytes: Vec<u8>,
    pub(super) catalog_bytes: Vec<u8>,
    pub(super) contract_bytes: Vec<u8>,
    pub(super) contract: OperationContractV1,
}

impl Fixture {
    /// Build the canonical direct execution-path fixture for one native harness.
    pub(super) fn direct(adapter: TestAdapter) -> Self {
        let strict_posture = match adapter {
            TestAdapter::ClaudeCode => FailurePosture::FailClosed,
            TestAdapter::Codex => FailurePosture::FailOpen,
        };
        Self::from_contract(adapter, baseline_contract(strict_posture))
    }

    /// Build a fixture whose witnessed path contains one delegation edge.
    pub(super) fn delegated(adapter: TestAdapter) -> Self {
        Self::from_contract_on_path(
            adapter,
            baseline_contract(FailurePosture::FailClosed),
            vec!["delegate-entry-worker".into()],
        )
    }

    /// Seal a fixture from an explicitly varied operation contract.
    pub(super) fn from_contract(adapter: TestAdapter, contract: OperationContractV1) -> Self {
        Self::from_contract_on_path(adapter, contract, Vec::new())
    }

    fn from_contract_on_path(
        adapter_kind: TestAdapter,
        mut contract: OperationContractV1,
        work_delegation_edge_ids: Vec<String>,
    ) -> Self {
        contract = OperationContractV1::seal(contract.to_preimage()).unwrap();
        pale_contracts::validate_record(&contract, "operation-contract.v1")
            .expect("operation contract fixture must satisfy the current exact schema");
        let contract_bytes = canonical_bytes(&contract);
        let profile = ProfileDigest::parse("b".repeat(64)).unwrap();
        let repository = LogicalRepositoryIdV1::parse("website-config")
            .expect("fixture repository identity is valid");
        let binding_body = adapter_kind.binding();
        let provider_binding = ProviderBindingRefV1 {
            provider_binding_id: binding_body.id.clone(),
            provider_binding_identity: binding_body.binding_identity.clone(),
        };
        let adapter = for_exact_binding(&provider_binding, &binding_body).unwrap();
        let schema_identity =
            pale_contracts::record_schema_identity("operation-catalog.v1").unwrap();
        let catalog = OperationContractCatalogV1::seal(OperationContractCatalogPreimageV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: 1,
            profile_digest: profile.clone(),
            repository_id: repository.clone(),
            provider_binding: provider_binding.clone(),
            entries: vec![OperationCatalogEntryV1::from_contract(
                &contract,
                artifact(".pale/operations/release.json", &contract_bytes),
            )],
        })
        .unwrap();
        pale_contracts::validate_record(&catalog, "operation-catalog.v1")
            .expect("operation catalog fixture must satisfy the current exact schema");
        let catalog_digest = catalog.catalog_digest.clone();
        let catalog_bytes = canonical_bytes(&catalog);
        let execution_path = ExecutionPathSelectionV1 {
            id: ExecutionPathSelectionIdV1::parse(match adapter_kind {
                TestAdapter::ClaudeCode => "direct-claude-code",
                TestAdapter::Codex => "direct-codex",
            })
            .expect("fixture execution-path identity is valid"),
            experience_path_id: "direct-owner".into(),
        };
        let execution_path_witness = ExecutionPathWitnessV1 {
            selection: execution_path,
            responsibility_path_identity: ResponsibilityPathIdentityV1 {
                responsibility_path_sha256: ArtifactDigest::hash_bytes(b"fixture-path"),
            },
            work_delegation_edge_ids,
            terminal_provider_binding: Some(provider_binding.clone()),
            composition_plan_digest: ExperienceCompositionPlanDigest::hash_canonical(
                &"fixture-composition-plan",
            )
            .expect("fixture composition-plan digest is canonical"),
        };
        let schema_identity =
            pale_contracts::record_schema_identity("operation-route-binding.v1").unwrap();
        let binding = OperationRouteBindingV1::seal(OperationRouteBindingPreimageV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: 1,
            profile_digest: profile.clone(),
            repository_id: repository.clone(),
            execution_path_witness: execution_path_witness.clone(),
            catalog: artifact(".pale/operation-catalog.json", &catalog_bytes),
            catalog_digest,
        })
        .unwrap();
        pale_contracts::validate_record(&binding, "operation-route-binding.v1")
            .expect("operation binding fixture must satisfy the current exact schema");
        let binding_bytes = canonical_bytes(&binding);
        let binding_reference = artifact(".pale/operation-route-binding.json", &binding_bytes);
        Self {
            adapter,
            adapter_kind,
            provider_binding,
            execution_path_witness,
            profile,
            repository,
            binding_reference,
            binding_bytes,
            catalog_bytes,
            contract_bytes,
            contract,
        }
    }

    /// Re-seal the fixture after changing one requirement's failure posture.
    pub(super) fn with_failure_posture(&self, requirement: &str, posture: FailurePosture) -> Self {
        self.with_capability(requirement, |capability| {
            capability.failure_posture = posture;
        })
    }

    /// Re-seal the fixture after changing one exact route-resolved requirement.
    pub(super) fn with_capability(
        &self,
        requirement: &str,
        update: impl FnOnce(&mut OperationCapabilityContractV1),
    ) -> Self {
        let mut contract = self.contract.clone();
        let OperationContractMechanicsV1::CapabilityBearing { capabilities } =
            &mut contract.mechanics
        else {
            panic!("the admission fixture is executable");
        };
        capabilities
            .iter_mut()
            .find(|capability| capability.requirement_id.as_str() == requirement)
            .map(update)
            .expect("fixture capability requirement exists");
        Self::from_contract(self.adapter_kind, contract)
    }

    /// Admit the fixture's sealed binding and catalog bytes.
    pub(super) fn bundle(&self) -> Result<VerifiedOperationBundle, OperationAdmissionError> {
        VerifiedOperationBundle::try_from_sealed_bytes(
            &self.binding_reference,
            &self.binding_bytes,
            &self.catalog_bytes,
            &self.profile,
            &self.repository,
            &self.execution_path_witness,
            self.adapter,
        )
    }

    /// Admit the fixture operation through its verified bundle.
    pub(super) fn operation<'a>(
        &self,
        bundle: &'a VerifiedOperationBundle,
    ) -> Result<AdmittedOperation<'a>, OperationAdmissionError> {
        bundle.admit_contract_bytes(&resource("release"), &self.contract_bytes)
    }

    /// Build the harness-specific owner event required by direct activation.
    pub(super) fn owner_event(&self, prompt: &str) -> (StdinEventFields, HookEvent) {
        let (payload, event) = match self.adapter_kind {
            TestAdapter::Codex => (
                serde_json::json!({
                    "hook_event_name": "UserPromptSubmit",
                    "prompt": prompt,
                    "session_id": "session-a",
                    "turn_id": "turn-a"
                }),
                HookEvent::UserPromptSubmit,
            ),
            TestAdapter::ClaudeCode => {
                let (command, arguments) = prompt
                    .trim()
                    .split_once(char::is_whitespace)
                    .unwrap_or((prompt.trim(), ""));
                (
                    serde_json::json!({
                        "hook_event_name": "UserPromptExpansion",
                        "expansion_type": "slash_command",
                        "command_name": command.trim_start_matches('/'),
                        "command_args": arguments,
                        "command_source": "project",
                        "prompt": prompt,
                        "session_id": "session-a",
                        "turn_id": "turn-a"
                    }),
                    HookEvent::UserPromptExpansion,
                )
            }
        };
        let bytes = serde_json::to_vec(&payload).unwrap();
        let StdinEvent::Fields(fields) = parse_stdin_event(&bytes, 1 << 20) else {
            panic!("owner event fixture must parse");
        };
        (*fields, event)
    }

    /// Build the exact model-facing invocation for this fixture operation.
    pub(super) fn invocation(&self, arguments: &str) -> String {
        let sigil = match self.adapter_kind {
            TestAdapter::ClaudeCode => '/',
            TestAdapter::Codex => '$',
        };
        format!("{sigil}pale-op-release{arguments}")
    }

    /// Activate the fixture through the production direct-owner boundary.
    pub(super) fn activate(
        &self,
        arguments: &str,
    ) -> Result<Option<SessionRuntimeState>, OperationAdmissionError> {
        let prompt = self.invocation(arguments);
        let (fields, event) = self.owner_event(&prompt);
        let proof = fields
            .direct_owner_prompt(event, &self.provider_binding, self.adapter)
            .ok_or(OperationAdmissionError::InvocationOriginMismatch)?;
        let bundle = self.bundle()?;
        let Some(selection) = bundle.select_direct_owner_prompt(proof)? else {
            return Ok(None);
        };
        let operation = self.operation(&bundle)?;
        let ready = operation.admit_activation_content(
            SKILL_BYTES,
            Some(POLICY_BYTES),
            [DEPENDENCY_BYTES],
        )?;
        let turn_digest = TurnIdentityDigest::try_from_host_identity("turn-a").unwrap();
        let activation = ready.activate(selection, Some(turn_digest))?;
        let mut state = SessionRuntimeState::default();
        state.activate_operation(activation)?;
        Ok(Some(state))
    }

    /// Admit one fixture tool invocation against the current lease.
    pub(super) fn admit_tool(
        &self,
        lease: &ActiveOperationLease,
        invocation: ToolInvocation<'_>,
    ) -> Result<AdmittedToolCall, OperationAdmissionError> {
        let bundle = self.bundle()?;
        self.operation(&bundle)?.admit_tool(lease, invocation)
    }

    /// Record one requirement failure through the production state transition.
    pub(super) fn record_failure(
        &self,
        lease: &ActiveOperationLease,
        invocation: ToolInvocation<'_>,
    ) -> Result<RecordedToolFailure, OperationAdmissionError> {
        let bundle = self.bundle()?;
        self.operation(&bundle)?
            .record_tool_failure(lease, invocation)
    }

    /// Attempt operation completion through the production terminal gate.
    pub(super) fn complete(
        &self,
        lease: &ActiveOperationLease,
        invocation: ToolInvocation<'_>,
    ) -> Result<CompletedToolCall, OperationAdmissionError> {
        let bundle = self.bundle()?;
        self.operation(&bundle)?
            .complete_tool_success(lease, invocation)
    }
}

/// Construct one capability requirement for admission tests.
pub(super) fn tool<'a>(
    fixture: &'a Fixture,
    tool_use_id: &'a str,
    tool_name: &'a str,
    tool_input: Option<&'a serde_json::Value>,
    instance: &'a InstanceDigest,
    turn_id: Option<&'a str>,
) -> ToolInvocation<'a> {
    ToolInvocation::try_from_host(ObservedToolInvocation {
        provider_binding: &fixture.provider_binding,
        adapter: fixture.adapter,
        tool_name,
        tool_use_id: Some(tool_use_id),
        tool_input,
        command: None,
        instance_digest: instance,
        session_id: Some("session-a"),
        turn_id,
    })
    .unwrap()
}

/// Clone the active fixture lease or fail the test with a focused message.
pub(super) fn current_lease(state: &SessionRuntimeState) -> ActiveOperationLease {
    state.active_operation().unwrap().clone()
}

/// Build the canonical structured publish-tool input.
pub(super) fn publish_input(target: &str) -> serde_json::Value {
    serde_json::json!({"mode": "publish", "target": target})
}

/// Build an activated state carrying exact authorization for the publish fixture.
pub(super) fn authorized_state(fixture: &Fixture) -> SessionRuntimeState {
    let pending = fixture.activate("").unwrap().unwrap();
    let lease = current_lease(&pending);
    let input = publish_input("release-1");
    let scope_digest = match fixture.admit_tool(
        &lease,
        tool(
            fixture,
            "challenge",
            "mcp__publisher__publish",
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    ) {
        Err(OperationAdmissionError::ToolAuthorizationRequired { scope_digest, .. }) => {
            scope_digest
        }
        result => panic!("expected exact authorization challenge, got {result:?}"),
    };
    fixture
        .activate(&format!(" --authorize publish-change={scope_digest}"))
        .unwrap()
        .unwrap()
}

/// Persist a successful pre-tool admission transition.
pub(super) fn apply_pre(
    fixture: &Fixture,
    state: &mut SessionRuntimeState,
    invocation: ToolInvocation<'_>,
) -> AdmittedToolCall {
    let expected = current_lease(state);
    let admitted = fixture.admit_tool(&expected, invocation).unwrap();
    assert!(admitted.apply_to(state, &expected).unwrap());
    admitted
}

/// Persist a post-tool failure transition for the matching invocation.
pub(super) fn apply_failure(
    fixture: &Fixture,
    state: &mut SessionRuntimeState,
    invocation: ToolInvocation<'_>,
) -> RecordedToolFailure {
    let expected = current_lease(state);
    let failure = fixture.record_failure(&expected, invocation).unwrap();
    assert!(failure.apply_to(state, &expected).unwrap());
    failure
}

/// Persist a post-tool success transition for the matching invocation.
pub(super) fn apply_success(
    fixture: &Fixture,
    state: &mut SessionRuntimeState,
    invocation: ToolInvocation<'_>,
) {
    let expected = current_lease(state);
    let completed = fixture.complete(&expected, invocation).unwrap();
    assert!(completed.apply_to(state, &expected).unwrap());
}

fn artifact(path: &str, bytes: &[u8]) -> SealedArtifactRef {
    SealedArtifactRef {
        rel_path: path.into(),
        sha256: ArtifactDigest::hash_bytes(bytes),
    }
}

fn pointer(value: &str) -> NonRootJsonPointer {
    NonRootJsonPointer::try_from(value).expect("fixture pointer is valid")
}

fn canonical_bytes<T: serde::Serialize>(value: &T) -> Vec<u8> {
    pale_core::canon::to_canonical_string(value)
        .unwrap()
        .into_bytes()
}

fn baseline_contract(failure_posture: FailurePosture) -> OperationContractV1 {
    let capability = |requirement_id: &str,
                      capability_id: &str,
                      effect_class: EffectClass,
                      approval: Approval,
                      provider: CapabilityProvider,
                      tool_name: &str,
                      input_equals| {
        let host_tools = vec![HostToolBinding {
            name: tool_name.into(),
            model_name: None,
            observability: ToolObservability::PreToolObservable,
            commands: vec![],
            command_source: None,
            input_equals,
        }];
        let provider = match provider {
            CapabilityProvider::HostNative => RouteCapabilityProviderV1::HostNative {
                execution: HostedProviderExecutionV1::Available { host_tools },
            },
            CapabilityProvider::NativeRust => RouteCapabilityProviderV1::NativeRust {
                target: "path-inventory".into(),
                execution: NativeRustProviderExecutionV1::Available {
                    host_tools,
                    native_action: "pale.compute.path-inventory".into(),
                },
            },
            CapabilityProvider::Connector => RouteCapabilityProviderV1::Connector {
                target: "public-query".into(),
                execution: HostedProviderExecutionV1::Available { host_tools },
            },
        };
        let policy = if approval == Approval::Gated {
            ResolvedCapabilityRequirementPolicyV1::RequiredBlock
        } else {
            ResolvedCapabilityRequirementPolicyV1::Optional {
                fallback: ResolvedCapabilityFallbackV1 {
                    guidance: "Continue without the optional capability.".into(),
                    recipe: None,
                },
            }
        };
        OperationCapabilityContractV1 {
            requirement_id: requirement(requirement_id),
            capability_id: CapabilityIdV1::parse(capability_id)
                .expect("test capability identity is valid"),
            recipe: None,
            effect_class,
            policy,
            failure_posture,
            provider_state: OperationCapabilityProviderStateV1::Bound {
                approval,
                provider,
                note: None,
            },
        }
    };
    let schema_identity = pale_contracts::record_schema_identity("operation-contract.v1").unwrap();
    OperationContractV1::seal(OperationContractPreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: 1,
        id: resource("release"),
        dispatch: OperationDispatchV1::UserOnly {
            invocation: "pale-op-release".into(),
            aliases: vec!["/release".into()],
        },
        mechanics: OperationContractMechanicsV1::CapabilityBearing {
            capabilities: vec![
                capability(
                    "digest-file",
                    "local-digest",
                    EffectClass::Compute,
                    Approval::None,
                    CapabilityProvider::NativeRust,
                    NATIVE_DIGEST_TOOL,
                    Default::default(),
                ),
                capability(
                    "inspect-remote",
                    "public-query",
                    EffectClass::Network,
                    Approval::Ask,
                    CapabilityProvider::HostNative,
                    "WebSearch",
                    Default::default(),
                ),
                capability(
                    "publish-change",
                    "remote-publish",
                    EffectClass::Publish,
                    Approval::Gated,
                    CapabilityProvider::HostNative,
                    "mcp__publisher__publish",
                    [(pointer("/mode"), "publish".into())].into(),
                ),
            ],
        },
        skill: artifact(".agents/skills/pale-op-release/SKILL.md", SKILL_BYTES),
        invocation_policy: Some(artifact(".pale/operations/release-policy.md", POLICY_BYTES)),
        semantic_dependencies: vec![OperationSemanticDependencyV1 {
            kind: SemanticDependencyKind::Doctrine,
            id: resource("review"),
            artifact: artifact(".agents/skills/pale-review/SKILL.md", DEPENDENCY_BYTES),
        }],
        auxiliary_tools: vec![HostToolBinding {
            name: "Skill".into(),
            model_name: None,
            observability: ToolObservability::PreToolObservable,
            commands: vec![],
            command_source: None,
            input_equals: [(pointer("/skill"), "pale-doctrine-release".into())].into(),
        }],
        roles: vec![resource("reviewer")],
    })
    .unwrap()
}
