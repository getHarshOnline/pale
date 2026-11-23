//! Complete rows shared by profile pages and authoring associations.
//!
//! The Rust SDK owns selection, policy meets, row order and explanation handles.
//! These copied values do not admit effects or establish native readiness.

use crate::{
    EffectiveProfileDeclaredSelection, EffectiveProfileFinding, EffectiveProfileNativeRealization,
    EffectiveProfilePathReason, EffectiveProfileProviderDisposition, EffectiveProfileSelection,
    OperationExposure, RequirementDisposition, WorkspaceSubjectValue,
};
use napi_derive::napi;
use pale_sdk::{
    EffectiveProfileEffectClass as ApiEffect, EffectiveProfileEntry as ApiEntry,
    EffectiveProfileEntryDetail as Detail, EffectiveProfileParticipant as ApiParticipant,
    EffectiveProfileProviderBindingRef as ApiBindingRef,
    EffectiveProfileRepositoryAccess as ApiAccess,
    EffectiveProfileResponsibility as ApiResponsibility,
};

/// Effect class in an admitted ceiling, not permission to execute an effect.
#[napi]
pub enum EffectiveProfileEffectClass {
    /// Pure caller-supplied computation.
    Compute,
    /// Non-mutating read.
    Read,
    /// Local command execution without remote write.
    Execute,
    /// Local state mutation.
    Write,
    /// Remote read.
    Network,
    /// Remote or outward-facing write.
    Publish,
}
impl EffectiveProfileEffectClass {
    fn from_api(value: ApiEffect) -> Self {
        match value {
            ApiEffect::Compute => Self::Compute,
            ApiEffect::Read => Self::Read,
            ApiEffect::Execute => Self::Execute,
            ApiEffect::Write => Self::Write,
            ApiEffect::Network => Self::Network,
            ApiEffect::Publish => Self::Publish,
        }
    }
}

/// Maximum repository access admitted by the workspace edge.
#[napi]
pub enum EffectiveProfileRepositoryAccess {
    /// Read-only ceiling.
    Read,
    /// Mutation remains subject to separate effect admission.
    Mutate,
}
impl EffectiveProfileRepositoryAccess {
    fn from_api(value: ApiAccess) -> Self {
        match value {
            ApiAccess::Read => Self::Read,
            ApiAccess::Mutate => Self::Mutate,
        }
    }
}

/// Exact participant identity, narrowed by its `kind` in JavaScript.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfileParticipant {
    /// Contract-facing interface.
    Interface {
        /// Interface identity.
        interface_id: String,
    },
    /// Independently owned native application.
    Application {
        /// Application identity.
        application_id: String,
    },
    /// Participant encoded by the initial-v1 `engine` wire variant.
    Engine {
        /// Stable identity carried by the `engine` wire variant.
        engine_id: String,
    },
    /// Neutral capability.
    Capability {
        /// Capability identity.
        capability_id: String,
    },
    /// Terminal execution leaf.
    TerminalLeaf {
        /// Terminal-leaf identity.
        terminal_leaf_id: String,
    },
    /// Plugin participant.
    Plugin {
        /// Plugin identity.
        plugin_id: String,
    },
    /// Skill participant.
    Skill {
        /// Skill identity.
        skill_id: String,
    },
}
impl EffectiveProfileParticipant {
    fn from_api(value: &ApiParticipant) -> Self {
        match value {
            ApiParticipant::Interface { interface_id } => Self::Interface {
                interface_id: interface_id.clone(),
            },
            ApiParticipant::Application { application_id } => Self::Application {
                application_id: application_id.clone(),
            },
            ApiParticipant::Engine { engine_id } => Self::Engine {
                engine_id: engine_id.clone(),
            },
            ApiParticipant::Capability { capability_id } => Self::Capability {
                capability_id: capability_id.clone(),
            },
            ApiParticipant::TerminalLeaf { terminal_leaf_id } => Self::TerminalLeaf {
                terminal_leaf_id: terminal_leaf_id.clone(),
            },
            ApiParticipant::Plugin { plugin_id } => Self::Plugin {
                plugin_id: plugin_id.clone(),
            },
            ApiParticipant::Skill { skill_id } => Self::Skill {
                skill_id: skill_id.clone(),
            },
        }
    }
}

/// Assigned responsibility with required pin fields only for an extension.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfileResponsibility {
    /// Human or agent communication.
    Communication,
    /// Organizational planning, delegation or acceptance.
    OrganizationalWork,
    /// Customer-relationship or business-owned state.
    CrmBusinessState,
    /// Engineering through its native execution owner.
    EngineeringExecution,
    /// Context or memory resolution.
    ContextMemory,
    /// Terminal harness or model-backed execution.
    TerminalHarnessExecution,
    /// Native event or outbox publication.
    NativeEventPublication,
    /// Native effect followed by owner read-back.
    NativeEffectReadBack,
    /// Separately governed extension responsibility.
    Extension {
        /// Governing vocabulary namespace.
        namespace_id: String,
        /// Extension contract identity.
        contract_id: String,
        /// Exact admitted schema member.
        schema_id: String,
        /// Digest of the exact schema bytes.
        schema_sha256: String,
    },
}
impl EffectiveProfileResponsibility {
    fn from_api(value: &ApiResponsibility) -> Self {
        match value {
            ApiResponsibility::Communication => Self::Communication,
            ApiResponsibility::OrganizationalWork => Self::OrganizationalWork,
            ApiResponsibility::CrmBusinessState => Self::CrmBusinessState,
            ApiResponsibility::EngineeringExecution => Self::EngineeringExecution,
            ApiResponsibility::ContextMemory => Self::ContextMemory,
            ApiResponsibility::TerminalHarnessExecution => Self::TerminalHarnessExecution,
            ApiResponsibility::NativeEventPublication => Self::NativeEventPublication,
            ApiResponsibility::NativeEffectReadBack => Self::NativeEffectReadBack,
            ApiResponsibility::Extension {
                namespace_id,
                contract_id,
                schema_id,
                schema_sha256,
            } => Self::Extension {
                namespace_id: namespace_id.clone(),
                contract_id: contract_id.clone(),
                schema_id: schema_id.clone(),
                schema_sha256: schema_sha256.clone(),
            },
        }
    }
}

/// Exact terminal provider reference, not evidence of provider availability.
#[napi]
pub struct EffectiveProfileProviderBindingRef {
    inner: ApiBindingRef,
}
impl EffectiveProfileProviderBindingRef {
    fn from_api(value: &ApiBindingRef) -> Self {
        Self {
            inner: value.clone(),
        }
    }
}
#[napi]
impl EffectiveProfileProviderBindingRef {
    /// Stable provider-binding locator.
    #[napi(getter)]
    pub fn provider_binding_id(&self) -> String {
        self.inner.provider_binding_id.clone()
    }
    /// Identity of the complete provider-binding body.
    #[napi(getter)]
    pub fn provider_binding_identity(&self) -> String {
        self.inner.provider_binding_identity.clone()
    }
}

/// Exact payload of one profile row, narrowed by its `kind` in JavaScript.
///
/// Only fields belonging to the selected variant are present. The terminal
/// provider reference is intrinsically nullable; empty lists remain lists.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum EffectiveProfileEntryDetail {
    /// Normalized selector before effective closure expansion.
    DeclaredSelection {
        /// Exact declared selector.
        selection: EffectiveProfileDeclaredSelection,
    },
    /// Responsibility contract and its selected-path proof.
    Responsibility {
        /// Responsibility contract identity.
        contract_id: String,
        /// Selected execution path.
        execution_path_id: String,
        /// Participant bounded by this contract.
        participant: EffectiveProfileParticipant,
        /// Assigned responsibilities in owner-provided order.
        responsibilities: Vec<EffectiveProfileResponsibility>,
        /// Admitted effect ceiling, not execution permission.
        authority_ceiling: Vec<EffectiveProfileEffectClass>,
        /// Ordered causal reasons.
        reason_steps: Vec<EffectiveProfilePathReason>,
        /// Copied causal selector, not an admitted comparison handle.
        subject: WorkspaceSubjectValue,
    },
    /// Expanded effective selection.
    Selection {
        /// Exact selected value.
        selection: EffectiveProfileSelection,
    },
    /// Private capability and effect narrowing.
    CapabilityNarrowing {
        /// Retained capabilities in owner-provided order.
        permitted_capabilities: Vec<String>,
        /// Explicitly forbidden capabilities.
        forbidden_capabilities: Vec<String>,
        /// Retained effect classes.
        permitted_effects: Vec<EffectiveProfileEffectClass>,
        /// Explicitly forbidden effect classes.
        forbidden_effects: Vec<EffectiveProfileEffectClass>,
    },
    /// Admitted Package-policy meet and its contributors.
    GoverningCeiling {
        /// Semantic scope of the meet.
        scope: String,
        /// Governed Package resource.
        subject_resource: String,
        /// Canonical contributors in owner-provided order.
        contributors: Vec<String>,
        /// Exact policies whose bodies were met.
        policies: Vec<String>,
        /// Capabilities retained by the meet.
        permitted_capabilities: Vec<String>,
        /// Capabilities forbidden by the meet.
        forbidden_capabilities: Vec<String>,
        /// Effects retained by the meet.
        permitted_effects: Vec<EffectiveProfileEffectClass>,
        /// Effects forbidden by the meet.
        forbidden_effects: Vec<EffectiveProfileEffectClass>,
    },
    /// Repository access and materialization order.
    Repository {
        /// Logical repository identity.
        repository_id: String,
        /// Zero-based materialization order.
        materialization_order: u32,
        /// Admitted access ceiling, not effect authorization.
        access_ceiling: EffectiveProfileRepositoryAccess,
    },
    /// Provider-owned repository projection.
    ProviderRepository {
        /// Logical repository identity.
        repository_id: String,
        /// Selected provider-binding locator.
        provider_binding_id: String,
        /// Opaque native coordinate, not a local path assertion.
        native_repository_coordinate: String,
        /// Separately qualified native evidence.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// Selected execution path and its ceilings.
    ExecutionPath {
        /// Stable path selection identity.
        selection_id: String,
        /// Selected experience path.
        experience_path_id: String,
        /// Exact responsibility-path identity.
        responsibility_path_identity: String,
        /// Consumed responsibility contracts in owner-provided order.
        responsibility_contract_ids: Vec<String>,
        /// Traversed neutral delegation edges.
        work_delegation_edge_ids: Vec<String>,
        /// Explicit provider bindings required by this path.
        provider_binding_ids: Vec<String>,
        /// Admitted capability ceiling.
        capability_ceiling: Vec<String>,
        /// Admitted effect ceiling.
        effect_ceiling: Vec<EffectiveProfileEffectClass>,
        /// Exact terminal binding, or null for a path without a terminal leaf.
        terminal_provider_binding: Option<EffectiveProfileProviderBindingRef>,
        /// Separately qualified native evidence.
        native_realization: EffectiveProfileNativeRealization,
        /// Ordered causal reasons.
        reason_steps: Vec<EffectiveProfilePathReason>,
        /// Copied causal selector, not an admitted comparison handle.
        subject: WorkspaceSubjectValue,
    },
    /// Configured operation exposure.
    Operation {
        /// Owning logical repository.
        repository_id: String,
        /// Owner-qualified operation identity.
        operation_id: String,
        /// Three-state private exposure.
        exposure: OperationExposure,
        /// Independent native evidence.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// Capability requirement and its independent disposition axes.
    Capability {
        /// Portable capability identity.
        capability_id: String,
        /// Owning logical repository.
        repository_id: String,
        /// Owner-qualified operation identity.
        operation_id: String,
        /// Requirement identity local to the operation.
        requirement_id: String,
        /// Effect required by this occurrence.
        effect_class: EffectiveProfileEffectClass,
        /// Portable configuration disposition.
        requirement_disposition: RequirementDisposition,
        /// Private provider disposition.
        provider_disposition: EffectiveProfileProviderDisposition,
        /// Independent native evidence.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// Separately qualified machine realization.
    MachineRealization {
        /// Selected machine-binding identity.
        machine_id: String,
        /// Machine-specific native evidence.
        realization: EffectiveProfileNativeRealization,
    },
    /// Application-owned finding.
    Finding {
        /// Exact finding value.
        finding: EffectiveProfileFinding,
    },
}

impl EffectiveProfileEntryDetail {
    fn from_api(value: &Detail) -> Self {
        match value {
            Detail::DeclaredSelection { selection } => Self::DeclaredSelection {
                selection: EffectiveProfileDeclaredSelection::from_api(selection),
            },
            Detail::Responsibility {
                contract_id,
                execution_path_id,
                participant,
                responsibilities,
                authority_ceiling,
                reason_steps,
                subject,
            } => Self::Responsibility {
                contract_id: contract_id.clone(),
                execution_path_id: execution_path_id.clone(),
                participant: EffectiveProfileParticipant::from_api(participant),
                responsibilities: responsibilities
                    .iter()
                    .map(EffectiveProfileResponsibility::from_api)
                    .collect(),
                authority_ceiling: authority_ceiling
                    .iter()
                    .copied()
                    .map(EffectiveProfileEffectClass::from_api)
                    .collect(),
                reason_steps: reason_steps
                    .iter()
                    .map(EffectiveProfilePathReason::from_api)
                    .collect(),
                subject: WorkspaceSubjectValue::from_api(subject),
            },
            Detail::Selection { selection } => Self::Selection {
                selection: EffectiveProfileSelection::from_api(selection),
            },
            Detail::CapabilityNarrowing {
                permitted_capabilities,
                forbidden_capabilities,
                permitted_effects,
                forbidden_effects,
            } => Self::CapabilityNarrowing {
                permitted_capabilities: permitted_capabilities.clone(),
                forbidden_capabilities: forbidden_capabilities.clone(),
                permitted_effects: permitted_effects
                    .iter()
                    .copied()
                    .map(EffectiveProfileEffectClass::from_api)
                    .collect(),
                forbidden_effects: forbidden_effects
                    .iter()
                    .copied()
                    .map(EffectiveProfileEffectClass::from_api)
                    .collect(),
            },
            Detail::GoverningCeiling {
                scope,
                subject_resource,
                contributors,
                policies,
                permitted_capabilities,
                forbidden_capabilities,
                permitted_effects,
                forbidden_effects,
            } => Self::GoverningCeiling {
                scope: scope.clone(),
                subject_resource: subject_resource.clone(),
                contributors: contributors.clone(),
                policies: policies.clone(),
                permitted_capabilities: permitted_capabilities.clone(),
                forbidden_capabilities: forbidden_capabilities.clone(),
                permitted_effects: permitted_effects
                    .iter()
                    .copied()
                    .map(EffectiveProfileEffectClass::from_api)
                    .collect(),
                forbidden_effects: forbidden_effects
                    .iter()
                    .copied()
                    .map(EffectiveProfileEffectClass::from_api)
                    .collect(),
            },
            Detail::Repository {
                repository_id,
                materialization_order,
                access_ceiling,
            } => Self::Repository {
                repository_id: repository_id.clone(),
                materialization_order: *materialization_order,
                access_ceiling: EffectiveProfileRepositoryAccess::from_api(*access_ceiling),
            },
            Detail::ProviderRepository {
                repository_id,
                provider_binding_id,
                native_repository_coordinate,
                native_realization,
            } => Self::ProviderRepository {
                repository_id: repository_id.clone(),
                provider_binding_id: provider_binding_id.clone(),
                native_repository_coordinate: native_repository_coordinate.clone(),
                native_realization: EffectiveProfileNativeRealization::from_api(
                    *native_realization,
                ),
            },
            Detail::ExecutionPath {
                selection_id,
                experience_path_id,
                responsibility_path_identity,
                responsibility_contract_ids,
                work_delegation_edge_ids,
                provider_binding_ids,
                capability_ceiling,
                effect_ceiling,
                terminal_provider_binding,
                native_realization,
                reason_steps,
                subject,
            } => Self::ExecutionPath {
                selection_id: selection_id.clone(),
                experience_path_id: experience_path_id.clone(),
                responsibility_path_identity: responsibility_path_identity.clone(),
                responsibility_contract_ids: responsibility_contract_ids.clone(),
                work_delegation_edge_ids: work_delegation_edge_ids.clone(),
                provider_binding_ids: provider_binding_ids.clone(),
                capability_ceiling: capability_ceiling.clone(),
                effect_ceiling: effect_ceiling
                    .iter()
                    .copied()
                    .map(EffectiveProfileEffectClass::from_api)
                    .collect(),
                terminal_provider_binding: terminal_provider_binding
                    .as_ref()
                    .map(EffectiveProfileProviderBindingRef::from_api),
                native_realization: EffectiveProfileNativeRealization::from_api(
                    *native_realization,
                ),
                reason_steps: reason_steps
                    .iter()
                    .map(EffectiveProfilePathReason::from_api)
                    .collect(),
                subject: WorkspaceSubjectValue::from_api(subject),
            },
            Detail::Operation {
                repository_id,
                operation_id,
                exposure,
                native_realization,
            } => Self::Operation {
                repository_id: repository_id.clone(),
                operation_id: operation_id.clone(),
                exposure: OperationExposure::from_api(*exposure),
                native_realization: EffectiveProfileNativeRealization::from_api(
                    *native_realization,
                ),
            },
            Detail::Capability {
                capability_id,
                repository_id,
                operation_id,
                requirement_id,
                effect_class,
                requirement_disposition,
                provider_disposition,
                native_realization,
            } => Self::Capability {
                capability_id: capability_id.clone(),
                repository_id: repository_id.clone(),
                operation_id: operation_id.clone(),
                requirement_id: requirement_id.clone(),
                effect_class: EffectiveProfileEffectClass::from_api(*effect_class),
                requirement_disposition: RequirementDisposition::from_api(*requirement_disposition),
                provider_disposition: EffectiveProfileProviderDisposition::from_api(
                    *provider_disposition,
                ),
                native_realization: EffectiveProfileNativeRealization::from_api(
                    *native_realization,
                ),
            },
            Detail::MachineRealization {
                machine_id,
                realization,
            } => Self::MachineRealization {
                machine_id: machine_id.clone(),
                realization: EffectiveProfileNativeRealization::from_api(*realization),
            },
            Detail::Finding { finding } => Self::Finding {
                finding: EffectiveProfileFinding::from_api(finding),
            },
        }
    }
}

/// One retained SDK row with its original opaque explanation handle.
///
/// Values remain readable independently of the page or join that returned
/// them. Explaining a handle still requires a retained matching profile.
#[napi]
pub struct EffectiveProfileEntry {
    inner: ApiEntry,
}
impl EffectiveProfileEntry {
    pub(crate) fn from_api(value: &ApiEntry) -> Self {
        Self {
            inner: value.clone(),
        }
    }
}

#[napi]
impl EffectiveProfileEntry {
    /// Original SDK-owned row explanation identity.
    #[napi(getter)]
    pub fn explanation_handle(&self) -> String {
        self.inner.explanation_handle().as_str().to_owned()
    }

    /// Copy this row's exact variant payload into an output-only value.
    ///
    /// Each read copies the selected payload, including its arrays. Retain the
    /// returned detail when inspecting several fields. Editing the JavaScript
    /// copy cannot change this retained row or admit a new subject or effect.
    #[napi(getter)]
    pub fn detail(&self) -> EffectiveProfileEntryDetail {
        EffectiveProfileEntryDetail::from_api(&self.inner.detail)
    }
}
