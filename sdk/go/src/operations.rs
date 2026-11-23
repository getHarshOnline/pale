//! Operation-specific translation into the one owning `pale-sdk` façade.

use base64::Engine as _;
use pale_sdk::{
    ApiLeaf, AuthoringInspectionRequest as ApiInspectionRequest,
    AuthoringNavigationRequest as ApiNavigationRequest, ContractDocument,
    DurablePublicationMechanism, LocalMaterializationAction, LocalMaterializationAdmissionRequest,
    LocalMaterializationDrift, LocalMaterializationMachineEffectKind,
    LocalMaterializationOwnershipChange, LocalMaterializationOwnershipObservation,
    LocalMaterializationReceiptObservation, LocalMaterializationRunOutcome,
    LocalMaterializationTargetOutcome, LocalNativeActionInputRequest,
    LocalOneShotCompilationRequest, LocalOneShotPlanRequest, LocalPackageDistributionRequest,
    LocalWorkspaceLocator, NativeActionPayloadRequest, NativeActionRequest, ObservationCausalPath,
    ObservationCausalSelector, ObservationFact, ObservationLens, ObservationLensDisposition,
    ObservationPage, ObservationPageDisposition, ObservationResynchronization, ObservationSubject,
    OneShotPlanKind, PackageDistributionRequest, RepositoryNativeActionCatalogRequest,
    ResidentHookRequest,
};
use serde::Serialize;

use crate::bridge_state;
use crate::wire;
use crate::{diagnostic, empty_success, encode, success, BridgeFailure};

pub(crate) fn api<T>(
    result: Result<T, pale_sdk::ApiError>,
    project: impl FnOnce(T) -> Result<crate::bridge_state::Response, BridgeFailure>,
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    Ok(match result {
        Ok(value) => project(value)?,
        Err(error) => diagnostic(error)?,
    })
}

pub(crate) fn json_response<T: Serialize>(
    value: &T,
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    Ok(success(encode(value)?, Vec::new(), 0))
}

pub(crate) fn inspect_local_control_store(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalControlStorageInspectionRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        api(
            client.api.inspect_local_control_store(request.into()),
            |inspection| json_response(&inspection),
        )
    })
}

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContractSchemaMetadata<'a> {
    registry_id: &'a str,
    schema_uri: &'a str,
    wire_schema_id: Option<&'a str>,
    schema_sha256: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidatedDocumentMetadata<'a> {
    schema_id: &'a str,
    schema_sha256: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IngressLimits {
    max_documents: usize,
    max_document_bytes: u64,
    max_total_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionDescriptor<'a> {
    action_id: &'a str,
    mcp_tool_name: &'a str,
    description: &'a str,
    contract_bytes: String,
    payload_schema_bytes: String,
}

fn action_descriptor(descriptor: &pale_sdk::NativeActionDescriptor) -> ActionDescriptor<'_> {
    ActionDescriptor {
        action_id: descriptor.action_id(),
        mcp_tool_name: descriptor.mcp_tool_name(),
        description: descriptor.description(),
        contract_bytes: base64(descriptor.canonical_contract_bytes()),
        payload_schema_bytes: base64(descriptor.canonical_payload_schema_bytes()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdmittedRepository<'a> {
    profile_digest: &'a str,
    instance_digest: &'a str,
    repository_id: &'a str,
    descriptors: Vec<ActionDescriptor<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ObservationSubjectHandle<'a> {
    fact_index: u32,
    handle: u64,
    subject: &'a ObservationSubject,
}

#[derive(Serialize)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum ObservationPageOutcome<'a> {
    Ready {
        page: &'a ObservationPage,
        subject_handles: Vec<ObservationSubjectHandle<'a>>,
        continuation_handle: Option<u64>,
    },
    Cancelled {
        continuation_handle: Option<u64>,
    },
    Resynchronize {
        resynchronization: ObservationResynchronization,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum ObservationLensOutcome<'a> {
    Ready {
        lens: &'a ObservationLens,
        continuation_handle: Option<u64>,
    },
    Resynchronize {
        resynchronization: &'a ObservationResynchronization,
    },
    Cancelled {
        continuation_handle: Option<u64>,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum ObservationCausalPathOutcome<'a> {
    Ready {
        path: &'a ObservationCausalPath,
    },
    Absent,
    Resynchronize {
        resynchronization: &'a ObservationResynchronization,
    },
}

fn observation_subject(fact: &ObservationFact) -> Option<&ObservationSubject> {
    match fact {
        ObservationFact::OperationExposure { subject, .. }
        | ObservationFact::CapabilityRequirementDisposition { subject, .. }
        | ObservationFact::Health { subject, .. }
        | ObservationFact::Readiness { subject, .. }
        | ObservationFact::Effect { subject, .. }
        | ObservationFact::NativeOwner { subject, .. }
        | ObservationFact::Reconciliation { subject, .. } => Some(subject),
        ObservationFact::ProcessIntent { .. } => None,
    }
}

fn plan_kind(kind: OneShotPlanKind) -> &'static str {
    match kind {
        OneShotPlanKind::Apply => "apply",
        OneShotPlanKind::Adopt => "adopt",
        OneShotPlanKind::Remove => "remove",
    }
}

fn materialization_action(action: LocalMaterializationAction) -> &'static str {
    match action {
        LocalMaterializationAction::Create => "create",
        LocalMaterializationAction::Adopt => "adopt",
        LocalMaterializationAction::Update => "update",
        LocalMaterializationAction::Delete => "delete",
        LocalMaterializationAction::Splice => "splice",
    }
}

fn ownership_change(change: LocalMaterializationOwnershipChange) -> &'static str {
    match change {
        LocalMaterializationOwnershipChange::Initialize => "initialize",
        LocalMaterializationOwnershipChange::Advance => "advance",
    }
}

fn machine_effect_kind(kind: LocalMaterializationMachineEffectKind) -> &'static str {
    match kind {
        LocalMaterializationMachineEffectKind::GitExcludeProtection => "git-exclude-protection",
    }
}

fn run_outcome(outcome: LocalMaterializationRunOutcome) -> &'static str {
    match outcome {
        LocalMaterializationRunOutcome::Succeeded => "succeeded",
        LocalMaterializationRunOutcome::Partial => "partial",
        LocalMaterializationRunOutcome::FailedNoEffect => "failed-no-effect",
    }
}

fn target_outcome(outcome: LocalMaterializationTargetOutcome) -> &'static str {
    match outcome {
        LocalMaterializationTargetOutcome::Succeeded => "succeeded",
        LocalMaterializationTargetOutcome::FailedNoEffect => "failed-no-effect",
    }
}

fn receipt_observation(observation: LocalMaterializationReceiptObservation) -> &'static str {
    match observation {
        LocalMaterializationReceiptObservation::NotConstructed => "not-constructed",
        LocalMaterializationReceiptObservation::Exact => "exact",
        LocalMaterializationReceiptObservation::Absent => "absent",
        LocalMaterializationReceiptObservation::Different => "different",
    }
}

fn ownership_observation(observation: LocalMaterializationOwnershipObservation) -> &'static str {
    match observation {
        LocalMaterializationOwnershipObservation::Unowned => "unowned",
        LocalMaterializationOwnershipObservation::MatchesPrior => "matches-prior",
        LocalMaterializationOwnershipObservation::MatchesPost => "matches-post",
        LocalMaterializationOwnershipObservation::Other => "other",
    }
}

fn durable_publication(publication: DurablePublicationMechanism) -> &'static str {
    match publication {
        DurablePublicationMechanism::AppleFullFsync => "apple-full-fsync",
        DurablePublicationMechanism::UnixFsync => "unix-fsync",
        DurablePublicationMechanism::StandardSyncAll => "standard-sync-all",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanCandidateMetadata<'a> {
    kind: &'static str,
    repository_id: &'a str,
    plan_digest: &'a str,
    one_shot_compilation_digest: &'a str,
    baseline_count: u32,
    operation_count: u32,
    create_count: u32,
    adopt_count: u32,
    update_count: u32,
    unchanged_count: u32,
    delete_count: u32,
    splice_count: u32,
    deferred_effect_count: u32,
    local_only_targets: &'a [String],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BoundPlanMetadata<'a> {
    kind: &'static str,
    repository_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdmittedPlanMetadata<'a> {
    kind: &'static str,
    repository_id: &'a str,
    plan_digest: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EffectiveAuthoringMetadata<'a> {
    coordinates: &'a pale_sdk::EffectiveProfileCoordinates,
    entries: &'a [pale_sdk::EffectiveProfileEntry],
    truncated: bool,
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum LocalDriftMetadata<'a> {
    Surface {
        relative_path: &'a str,
        action: &'static str,
    },
    Ownership {
        change: &'static str,
    },
    MachineLocal {
        effect: &'static str,
        relative_path: &'a str,
        action: &'static str,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalRepositoryStatusMetadata<'a> {
    repository_id: &'a str,
    clean: bool,
    drift: Vec<LocalDriftMetadata<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalPlanMetadata<'a> {
    kind: &'static str,
    repository_id: &'a str,
    baseline_count: u32,
    operation_count: u32,
    total_operation_count: u32,
    create_count: u32,
    adopt_count: u32,
    update_count: u32,
    unchanged_count: u32,
    delete_count: u32,
    splice_count: u32,
    deferred_effect_count: u32,
    machine_local_effect_count: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalPlanPublicationMetadata<'a> {
    repository_id: &'a str,
    relative_target: &'a str,
    display_target: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalRunTargetAdmissionMetadata<'a> {
    repository_id: &'a str,
    plan_digest: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalRunAdmissionMetadata<'a> {
    kind: &'static str,
    targets: Vec<LocalRunTargetAdmissionMetadata<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalRunTargetMetadata<'a> {
    repository_id: &'a str,
    outcome: &'static str,
    changed_surface_count: u32,
    deferred_effect_count: u32,
    local_git_exclude_changed: bool,
    failure: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalRunResultMetadata<'a> {
    kind: &'static str,
    outcome: &'static str,
    target_count: u32,
    succeeded_target_count: u32,
    targets: Vec<LocalRunTargetMetadata<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalReconciliationMetadata<'a> {
    repository_id: &'a str,
    plan_digest: &'a str,
    receipt: &'static str,
    ownership: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompiledInterfaceIdentityMetadata<'a> {
    version: &'a str,
    protocol: &'a str,
    os: &'a str,
    arch: &'a str,
    durable_publication: &'static str,
}

pub(crate) fn capabilities(
    client: u64,
    _request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    bridge_state::with_client(client, |_| {
        let ids = pale_sdk::capability_inventory()
            .iter()
            .filter(|capability| capability.eligible_for(ApiLeaf::Go))
            .map(|capability| capability.id().as_str())
            .collect::<Vec<_>>();
        json_response(&ids)
    })
}

pub(crate) fn contract_bundle_identity(
    client: u64,
    _request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    bridge_state::with_client(client, |client| {
        Ok(success(
            client
                .api
                .contract_bundle_identity()
                .canonical_bytes()
                .to_vec(),
            Vec::new(),
            0,
        ))
    })
}

pub(crate) fn contract_schema(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::SchemaRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        api(client.api.contract_schema(&request.schema_id), |schema| {
            Ok(success(
                encode(&ContractSchemaMetadata {
                    registry_id: schema.registry_id(),
                    schema_uri: schema.schema_uri(),
                    wire_schema_id: schema.wire_schema_id(),
                    schema_sha256: schema.schema_sha256(),
                })?,
                schema.bytes().to_vec(),
                0,
            ))
        })
    })
}

pub(crate) fn validate_contract_document(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ContractDocumentRequest = crate::decode(request)?;
    let bytes = wire::decode_bytes(&request.bytes)?;
    bridge_state::with_client(client, |client| {
        api(
            client
                .api
                .validate_contract_document(ContractDocument::new(request.schema_id, bytes)),
            |document| {
                Ok(success(
                    encode(&ValidatedDocumentMetadata {
                        schema_id: document.schema_id(),
                        schema_sha256: document.schema_sha256(),
                    })?,
                    document.canonical_bytes().to_vec(),
                    0,
                ))
            },
        )
    })
}

pub(crate) fn project_diagnostic(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::DiagnosticRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        api(client.api.project_diagnostic(request.into_api()), |value| {
            Ok(success(Vec::new(), value.canonical_bytes().to_vec(), 0))
        })
    })
}

pub(crate) fn admit_diagnostic(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::BytesRequest = crate::decode(request)?;
    let bytes = wire::decode_bytes(&request.bytes)?;
    bridge_state::with_client(client, |client| {
        api(client.api.admit_diagnostic(&bytes), |value| {
            Ok(success(Vec::new(), value.canonical_bytes().to_vec(), 0))
        })
    })
}

pub(crate) fn authoring_ingress_limits(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let class: wire::IngressClass = crate::decode(request)?;
    bridge_state::with_client(client, |_| {
        let limits = pale_sdk::PaleApi::authoring_ingress_limits(class.into());
        json_response(&IngressLimits {
            max_documents: limits.max_documents(),
            max_document_bytes: limits.max_document_bytes(),
            max_total_bytes: limits.max_total_bytes(),
        })
    })
}

pub(crate) fn validate_authoring_document(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::StandaloneValidationRequest = crate::decode(request)?;
    let bytes = wire::decode_bytes(&request.bytes)?;
    bridge_state::with_client(client, |client| {
        api(
            client.api.validate_authoring_document(
                request.role.into(),
                request.source_label,
                bytes,
            ),
            |_| Ok(empty_success()),
        )
    })
}

pub(crate) fn inspect_standalone_authoring(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::StandaloneInspectionRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        api(
            client.api.inspect_standalone_authoring(
                request.role.into(),
                request.path.into_iter().map(Into::into).collect(),
            ),
            |inspection| json_response(&inspection),
        )
    })
}

pub(crate) fn compile_portable_workspace(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::PortableRequest = crate::decode(request)?;
    let request = request.into_api()?;
    bridge_state::with_client(client, |state| {
        api(state.api.compile_portable_workspace(request), |workspace| {
            let metadata = workspace.summary().canonical_bytes().to_vec();
            let object = bridge_state::next_object_handle();
            let mut response = success(metadata, Vec::new(), object);
            response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                client,
                values: bridge_state::PendingResponseValues::PortableWorkspace {
                    handle: object,
                    workspace,
                },
            }));
            Ok(response)
        })
    })
}

pub(crate) fn inspect_portable_workspace_authoring(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::AuthoringInspectionRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let workspace = client
            .workspaces
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("portable workspace", request.handle))?;
        api(
            workspace.inspect_authoring(ApiInspectionRequest::new(
                request.source_label,
                request.path.into_iter().map(Into::into).collect(),
            )),
            |inspection| json_response(&inspection),
        )
    })
}

pub(crate) fn navigate_portable_workspace_authoring(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::AuthoringNavigationRequest = crate::decode(request)?;
    bridge_state::with_client(client, |state| {
        let navigation = state
            .workspaces
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("portable workspace", request.handle))?
            .navigate_authoring(ApiNavigationRequest::new(
                request.source_label,
                request.path.into_iter().map(Into::into).collect(),
            ));
        authoring_navigation_response(client, navigation)
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthoringAlternativeHandle {
    alternative_index: usize,
    handle: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthoringNavigationMetadata<'a> {
    navigation: &'a pale_sdk::AuthoringNavigation,
    alternative_handles: Vec<AuthoringAlternativeHandle>,
}

// Both navigation routes copy the same SDK result. The response owns all minted
// identities until Go decodes the complete value and claims their custody.
fn authoring_navigation_response(
    client: u64,
    navigation: pale_sdk::AuthoringNavigation,
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let mut identities = Vec::with_capacity(navigation.alternatives().len() + 1);
    let object = navigation.identity().map_or(0, |identity| {
        let handle = bridge_state::next_object_handle();
        identities.push((handle, identity.clone()));
        handle
    });
    let alternative_handles = navigation
        .alternatives()
        .iter()
        .enumerate()
        .map(|(alternative_index, identity)| {
            let handle = bridge_state::next_object_handle();
            identities.push((handle, identity.clone()));
            AuthoringAlternativeHandle {
                alternative_index,
                handle,
            }
        })
        .collect();
    let metadata = encode(&AuthoringNavigationMetadata {
        navigation: &navigation,
        alternative_handles,
    })?;
    let mut response = success(metadata, Vec::new(), object);
    if !identities.is_empty() {
        response.pending_values = Some(Box::new(bridge_state::PendingResponse {
            client,
            values: bridge_state::PendingResponseValues::AuthoringIdentities(identities),
        }));
    }
    Ok(response)
}

pub(crate) fn compare_portable_workspaces(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ComparisonRequest = crate::decode(request)?;
    bridge_state::with_client(client, |state| {
        let prior = state
            .workspaces
            .get(&request.prior)
            .ok_or_else(|| BridgeFailure::invalid_handle("portable workspace", request.prior))?;
        let current = state
            .workspaces
            .get(&request.current)
            .ok_or_else(|| BridgeFailure::invalid_handle("portable workspace", request.current))?;
        api(
            state.api.compare_portable_workspaces(prior, current),
            |comparison| {
                let metadata = comparison.summary().canonical_bytes().to_vec();
                let object = bridge_state::next_object_handle();
                let mut response = success(metadata, Vec::new(), object);
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::WorkspaceComparison {
                        handle: object,
                        comparison,
                    },
                }));
                Ok(response)
            },
        )
    })
}

pub(crate) fn portable_workspace_change_roots(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::PageRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let comparison = client
            .comparisons
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("workspace comparison", request.handle))?;
        Ok(success(
            Vec::new(),
            comparison
                .change_roots(request.offset, request.limit)
                .canonical_bytes()
                .to_vec(),
            0,
        ))
    })
}

pub(crate) fn portable_workspace_impacts(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::PageRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let comparison = client
            .comparisons
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("workspace comparison", request.handle))?;
        Ok(success(
            Vec::new(),
            comparison
                .impacts(request.offset, request.limit)
                .canonical_bytes()
                .to_vec(),
            0,
        ))
    })
}

pub(crate) fn compare_portable_workspace_subject(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::WorkspaceExplanationRequest = crate::decode(request)?;
    let selector = request.subject.into();
    bridge_state::with_client(client, |client| {
        let comparison = client
            .comparisons
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("workspace comparison", request.handle))?;
        api(comparison.compare_subject_selector(selector), |result| {
            Ok(success(Vec::new(), result.canonical_bytes().to_vec(), 0))
        })
    })
}

pub(crate) fn explain_portable_workspace(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::WorkspaceExplanationRequest = crate::decode(request)?;
    let selector = request.subject.into();
    bridge_state::with_client(client, |client| {
        let comparison = client
            .comparisons
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("workspace comparison", request.handle))?;
        api(comparison.explain_selector(selector), |explanation| {
            Ok(success(
                Vec::new(),
                explanation.canonical_bytes().to_vec(),
                0,
            ))
        })
    })
}

pub(crate) fn compile_effective_profile(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::EffectiveProfileRequest = crate::decode(request)?;
    let workspace_handle = request.workspace;
    let request = request.into_api()?;
    bridge_state::with_client(client, |state| {
        let workspace = state
            .workspaces
            .get(&workspace_handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("portable workspace", workspace_handle))?;
        api(
            state.api.compile_effective_profile(workspace, request),
            |profile| {
                let metadata = encode(&profile.coordinates())?;
                let object = bridge_state::next_object_handle();
                let mut response = success(metadata, Vec::new(), object);
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::EffectiveProfile {
                        handle: object,
                        profile,
                    },
                }));
                Ok(response)
            },
        )
    })
}

pub(crate) fn inspect_effective_profile(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ProfilePageRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let profile = client
            .profiles
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.handle))?;
        Ok(success(
            Vec::new(),
            profile
                .inspect(request.selector())
                .canonical_bytes()
                .to_vec(),
            0,
        ))
    })
}

pub(crate) fn explain_effective_profile(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ProfileExplanationRequest = crate::decode(request)?;
    let explanation = request.explanation()?;
    bridge_state::with_client(client, |client| {
        let profile = client
            .profiles
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.handle))?;
        json_response(&profile.explain(&explanation))
    })
}

pub(crate) fn inspect_effective_profile_authoring(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::AuthoringInspectionRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let profile = client
            .profiles
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.handle))?;
        api(
            profile.inspect_authoring(ApiInspectionRequest::new(
                request.source_label,
                request.path.into_iter().map(Into::into).collect(),
            )),
            |inspection| json_response(&inspection),
        )
    })
}

pub(crate) fn navigate_effective_profile_authoring(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::AuthoringNavigationRequest = crate::decode(request)?;
    bridge_state::with_client(client, |state| {
        let navigation = state
            .profiles
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.handle))?
            .navigate_authoring(ApiNavigationRequest::new(
                request.source_label,
                request.path.into_iter().map(Into::into).collect(),
            ));
        authoring_navigation_response(client, navigation)
    })
}

pub(crate) fn join_effective_profile_authoring(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ProfileAuthoringJoinRequest = crate::decode(request)?;
    let identity_handle = request.identity;
    bridge_state::with_client(client, |client| {
        let profile = client
            .profiles
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.handle))?;
        let identity = client
            .authoring_identities
            .get(&identity_handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("authoring identity", identity_handle))?;
        let inspection = profile.inspect_effective_authoring(identity);
        json_response(&EffectiveAuthoringMetadata {
            coordinates: inspection.coordinates(),
            entries: inspection.entries(),
            truncated: inspection.truncated(),
        })
    })
}

pub(crate) fn observe_effective_profile(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ObserveEffectiveProfileRequest = crate::decode(request)?;
    bridge_state::with_client(client, |state| {
        let profile = state
            .profiles
            .get(&request.profile)
            .ok_or_else(|| BridgeFailure::invalid_handle("effective profile", request.profile))?;
        let snapshot = state.api.observe_effective_profile(profile);
        let metadata = encode(&snapshot.summary())?;
        let object = bridge_state::next_object_handle();
        let mut response = success(metadata, Vec::new(), object);
        response.pending_values = Some(Box::new(bridge_state::PendingResponse {
            client,
            values: bridge_state::PendingResponseValues::Snapshot {
                handle: object,
                snapshot,
            },
        }));
        Ok(response)
    })
}

pub(crate) fn observation_page(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ObservationPageRequest = crate::decode(request)?;
    bridge_state::with_client(client, |state| {
        let (snapshot_handle, request) = request.into_api(state)?;
        let outcome = state
            .observation_snapshots
            .get(&snapshot_handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("observation snapshot", snapshot_handle))?
            .page(request);
        match outcome {
            Ok(ObservationPageDisposition::Ready { page }) => {
                let continuation = page
                    .continuation
                    .as_ref()
                    .map(|cursor| (bridge_state::next_object_handle(), Box::new(cursor.clone())));
                let subjects = page
                    .facts
                    .iter()
                    .enumerate()
                    .filter_map(|(index, fact)| {
                        observation_subject(fact).map(|subject| {
                            (
                                u32::try_from(index)
                                    .expect("bounded observation pages fit the Go index width"),
                                bridge_state::next_object_handle(),
                                subject.clone(),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                let subject_handles = subjects
                    .iter()
                    .map(|(fact_index, handle, subject)| ObservationSubjectHandle {
                        fact_index: *fact_index,
                        handle: *handle,
                        subject,
                    })
                    .collect();
                let mut response = json_response(&ObservationPageOutcome::Ready {
                    page: &page,
                    subject_handles,
                    continuation_handle: continuation.as_ref().map(|(handle, _)| *handle),
                })?;
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::Page {
                        continuation,
                        subjects: subjects
                            .into_iter()
                            .map(|(_, handle, subject)| (handle, subject))
                            .collect(),
                    },
                }));
                Ok(response)
            }
            Ok(ObservationPageDisposition::Cancelled { continuation }) => {
                let continuation = continuation
                    .map(|cursor| (bridge_state::next_object_handle(), Box::new(cursor)));
                let mut response = json_response(&ObservationPageOutcome::Cancelled {
                    continuation_handle: continuation.as_ref().map(|(handle, _)| *handle),
                })?;
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::Page {
                        continuation,
                        subjects: Vec::new(),
                    },
                }));
                Ok(response)
            }
            Err(resynchronization) => {
                json_response(&ObservationPageOutcome::Resynchronize { resynchronization })
            }
        }
    })
}

pub(crate) fn observation_delta(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ObservationDeltaRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let (current_handle, prior_handle, request) = request.into_api(client)?;
        let current = client
            .observation_snapshots
            .get(&current_handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("observation snapshot", current_handle))?;
        let prior = client
            .observation_snapshots
            .get(&prior_handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("observation snapshot", prior_handle))?;
        json_response(&current.delta_from(prior, request))
    })
}

pub(crate) fn observation_causal_path(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ObservationCausalPathRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let snapshot = client
            .observation_snapshots
            .get(&request.snapshot)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("observation snapshot", request.snapshot)
            })?;
        let subject = client
            .observation_subjects
            .get(&request.subject)
            .ok_or_else(|| BridgeFailure::invalid_handle("observation subject", request.subject))?;
        let selector = ObservationCausalSelector::new(snapshot, subject);
        let result = snapshot.causal_path(&selector);
        let outcome = match &result {
            Ok(Some(path)) => ObservationCausalPathOutcome::Ready { path },
            Ok(None) => ObservationCausalPathOutcome::Absent,
            Err(resynchronization) => {
                ObservationCausalPathOutcome::Resynchronize { resynchronization }
            }
        };
        json_response(&outcome)
    })
}

pub(crate) fn observation_visual_lens(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ObservationLensRequest = crate::decode(request)?;
    bridge_state::with_client(client, |state| {
        let (snapshot_handle, request) = request.into_api(state)?;
        let snapshot = state
            .observation_snapshots
            .get(&snapshot_handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("observation snapshot", snapshot_handle)
            })?;
        let disposition = snapshot.visual_lens(request);
        match &disposition {
            ObservationLensDisposition::Ready { lens } => {
                let continuation = lens
                    .continuation
                    .as_ref()
                    .map(|cursor| (bridge_state::next_object_handle(), Box::new(cursor.clone())));
                let mut response = json_response(&ObservationLensOutcome::Ready {
                    lens,
                    continuation_handle: continuation.as_ref().map(|(handle, _)| *handle),
                })?;
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::Lens { continuation },
                }));
                Ok(response)
            }
            ObservationLensDisposition::Resynchronize { resynchronization } => {
                json_response(&ObservationLensOutcome::Resynchronize { resynchronization })
            }
            ObservationLensDisposition::Cancelled { continuation } => {
                let continuation = continuation
                    .as_ref()
                    .map(|cursor| (bridge_state::next_object_handle(), Box::new(cursor.clone())));
                let mut response = json_response(&ObservationLensOutcome::Cancelled {
                    continuation_handle: continuation.as_ref().map(|(handle, _)| *handle),
                })?;
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::Lens { continuation },
                }));
                Ok(response)
            }
        }
    })
}

pub(crate) fn native_action_catalog(
    client: u64,
    _request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    bridge_state::with_client(client, |client| {
        api(client.api.native_action_descriptors(), |descriptors| {
            json_response(
                &descriptors
                    .iter()
                    .map(action_descriptor)
                    .collect::<Vec<_>>(),
            )
        })
    })
}

pub(crate) fn invoke_native_action(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ActionRequest = crate::decode(request)?;
    let bytes = wire::decode_bytes(&request.bytes)?;
    bridge_state::with_client(client, |client| {
        api(
            client
                .api
                .invoke_native_action(NativeActionRequest::new(request.action_id, bytes)),
            |execution| Ok(success(Vec::new(), execution.canonical_bytes().to_vec(), 0)),
        )
    })
}

pub(crate) fn invoke_native_action_payload(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ActionRequest = crate::decode(request)?;
    let bytes = wire::decode_bytes(&request.bytes)?;
    bridge_state::with_client(client, |client| {
        api(
            client
                .api
                .invoke_native_action_payload(NativeActionPayloadRequest::new(
                    request.action_id,
                    bytes,
                )),
            |execution| Ok(success(Vec::new(), execution.canonical_bytes().to_vec(), 0)),
        )
    })
}

pub(crate) fn admit_repository_native_actions(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::RepositoryActionRequest = crate::decode(request)?;
    let bytes = wire::decode_bytes(&request.bytes)?;
    bridge_state::with_client(client, |client| {
        api(
            client
                .api
                .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
                    bytes,
                    request.profile_digest,
                    request.instance_digest,
                    request.repository_id,
                )),
            |admitted| {
                let descriptors = admitted
                    .descriptors()
                    .iter()
                    .map(action_descriptor)
                    .collect();
                json_response(&AdmittedRepository {
                    profile_digest: admitted.profile_digest(),
                    instance_digest: admitted.instance_digest(),
                    repository_id: admitted.repository_id(),
                    descriptors,
                })
            },
        )
    })
}

pub(crate) fn compile_one_shot(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::OneShotRequest = crate::decode(request)?;
    let request = request.into_api()?;
    bridge_state::with_client(client, |state| {
        api(state.api.compile_one_shot(request), |compilation| {
            let metadata = compilation.summary().canonical_bytes().to_vec();
            let object = bridge_state::next_object_handle();
            let mut response = success(metadata, Vec::new(), object);
            response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                client,
                values: bridge_state::PendingResponseValues::OneShotCompilation {
                    handle: object,
                    compilation,
                },
            }));
            Ok(response)
        })
    })
}

pub(crate) fn one_shot_summary(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::HandleRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let compilation = client
            .compilations
            .get(&request.handle)
            .ok_or_else(|| BridgeFailure::invalid_handle("one-shot compilation", request.handle))?;
        Ok(success(
            compilation.summary().canonical_bytes().to_vec(),
            Vec::new(),
            0,
        ))
    })
}

pub(crate) fn one_shot_observation_targets(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ObservationRequest = crate::decode(request)?;
    let compilation_handle = request.compilation;
    let prior = request.prior.into_api()?;
    bridge_state::with_client(client, |client| {
        let compilation = client
            .compilations
            .get(&compilation_handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("one-shot compilation", compilation_handle)
            })?;
        api(
            client.api.one_shot_observation_targets(
                compilation,
                request.repository_id,
                request.kind.into(),
                &prior,
            ),
            |targets| Ok(success(Vec::new(), targets.canonical_bytes().to_vec(), 0)),
        )
    })
}

pub(crate) fn plan_one_shot(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::PlanningRequest = crate::decode(request)?;
    let compilation_handle = request.compilation;
    let request = request.request()?;
    bridge_state::with_client(client, |state| {
        let compilation = state.compilations.get(&compilation_handle).ok_or_else(|| {
            BridgeFailure::invalid_handle("one-shot compilation", compilation_handle)
        })?;
        api(state.api.plan_one_shot(compilation, request), |candidate| {
            let metadata = encode(&PlanCandidateMetadata {
                kind: plan_kind(candidate.kind()),
                repository_id: candidate.repository_id(),
                plan_digest: candidate.plan_digest(),
                one_shot_compilation_digest: candidate.one_shot_compilation_digest(),
                baseline_count: candidate.baseline_count(),
                operation_count: candidate.operation_count(),
                create_count: candidate.create_count(),
                adopt_count: candidate.adopt_count(),
                update_count: candidate.update_count(),
                unchanged_count: candidate.unchanged_count(),
                delete_count: candidate.delete_count(),
                splice_count: candidate.splice_count(),
                deferred_effect_count: candidate.deferred_effect_count(),
                local_only_targets: candidate.local_only_targets(),
            })?;
            let payload = candidate.canonical_bytes().to_vec();
            let object = bridge_state::next_object_handle();
            let mut response = success(metadata, payload, object);
            response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                client,
                values: bridge_state::PendingResponseValues::OneShotCandidate {
                    handle: object,
                    candidate,
                },
            }));
            Ok(response)
        })
    })
}

pub(crate) fn bind_one_shot_machine_effects(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::BindPlanRequest = crate::decode(request)?;
    let candidate_handle = request.candidate;
    let effects = request.effects()?;
    bridge_state::with_client_mut(client, |state| {
        let candidate = state.candidates.remove(&candidate_handle).ok_or_else(|| {
            BridgeFailure::invalid_handle("one-shot plan candidate", candidate_handle)
        })?;
        api(
            state.api.bind_one_shot_machine_effects(candidate, effects),
            |bound| {
                let metadata = encode(&BoundPlanMetadata {
                    kind: plan_kind(bound.kind()),
                    repository_id: bound.repository_id(),
                })?;
                let payload = bound.canonical_bytes().to_vec();
                let object = bridge_state::next_object_handle();
                let mut response = success(metadata, payload, object);
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::OneShotBoundPlan {
                        handle: object,
                        plan: bound,
                    },
                }));
                Ok(response)
            },
        )
    })
}

pub(crate) fn admit_one_shot_saved_plan(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::AdmitPlanRequest = crate::decode(request)?;
    let saved_bytes = wire::decode_bytes(&request.saved_bytes)?;
    bridge_state::with_client_mut(client, |state| {
        let rebuilt = state
            .bound_plans
            .remove(&request.bound_plan)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("one-shot bound plan", request.bound_plan)
            })?;
        api(
            state.api.admit_one_shot_saved_plan(rebuilt, &saved_bytes),
            |admitted| {
                let metadata = encode(&AdmittedPlanMetadata {
                    kind: plan_kind(admitted.kind()),
                    repository_id: admitted.repository_id(),
                    plan_digest: admitted.plan_digest(),
                })?;
                let object = bridge_state::next_object_handle();
                let mut response = success(metadata, Vec::new(), object);
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::OneShotAdmittedPlan {
                        handle: object,
                        plan: admitted,
                    },
                }));
                Ok(response)
            },
        )
    })
}

pub(crate) fn inspect_package_distribution(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::PackageDistributionRequest = crate::decode(request)?;
    let documents = request
        .documents
        .into_iter()
        .map(wire::Document::compilation)
        .collect::<Result<Vec<_>, _>>()?;
    bridge_state::with_client(client, |client| {
        api(
            client
                .api
                .inspect_package_distribution(PackageDistributionRequest::new(documents)),
            |inspection| {
                Ok(success(
                    Vec::new(),
                    inspection.canonical_bytes().to_vec(),
                    0,
                ))
            },
        )
    })
}

pub(crate) fn admit_resident_hook(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::ResidentHookRequest = crate::decode(request)?;
    let payload = wire::decode_bytes(&request.payload)?;
    bridge_state::with_client_mut(client, |client| {
        api(
            client.api.admit_resident_hook(ResidentHookRequest::new(
                request.event,
                request.workspace,
                request.repository,
                request.provider_binding,
                request.provider_binding_identity,
                payload,
            )),
            |admission| {
                let object = bridge_state::next_object_handle();
                client.resident_hook_admissions.insert(object, admission);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

fn local_selection(
    request: wire::LocalWorkspaceRequest,
) -> (LocalWorkspaceLocator, String, String, String) {
    (
        LocalWorkspaceLocator::new(request.manifest),
        request.project_id,
        request.execution_workspace_id,
        request.machine_id,
    )
}

pub(crate) fn compile_local_one_shot(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalWorkspaceRequest = crate::decode(request)?;
    let (workspace, project, execution_workspace, machine) = local_selection(request);
    bridge_state::with_client(client, |state| {
        api(
            state
                .api
                .compile_local_one_shot(LocalOneShotCompilationRequest::new(
                    workspace,
                    project,
                    execution_workspace,
                    machine,
                )),
            |compilation| {
                let metadata = compilation.summary().canonical_bytes().to_vec();
                let object = bridge_state::next_object_handle();
                let mut response = success(metadata, Vec::new(), object);
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::LocalCompilation {
                        handle: object,
                        compilation,
                    },
                }));
                Ok(response)
            },
        )
    })
}

pub(crate) fn preview_local_configuration_improvement(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalConfigurationImprovementRequest = crate::decode(request)?;
    bridge_state::with_client_mut(client, |client| {
        api(
            client.api.preview_local_configuration_improvement(
                pale_sdk::LocalConfigurationImprovementRequest::new(
                    LocalWorkspaceLocator::new(request.baseline_manifest),
                    LocalWorkspaceLocator::new(request.candidate_manifest),
                    request.project_id,
                    request.execution_workspace_id,
                    request.machine_id,
                ),
            ),
            |preview| {
                let object = bridge_state::next_object_handle();
                client.improvement_previews.insert(object, preview);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn inspect_local_package_distribution(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalWorkspaceRequest = crate::decode(request)?;
    let (workspace, project, execution_workspace, machine) = local_selection(request);
    bridge_state::with_client(client, |client| {
        api(
            client
                .api
                .inspect_local_package_distribution(LocalPackageDistributionRequest::new(
                    workspace,
                    project,
                    execution_workspace,
                    machine,
                )),
            |inspection| {
                Ok(success(
                    Vec::new(),
                    inspection.canonical_bytes().to_vec(),
                    0,
                ))
            },
        )
    })
}

pub(crate) fn capture_local_native_action_request(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalActionInputRequest = crate::decode(request)?;
    bridge_state::with_client_mut(client, |client| {
        api(
            client
                .api
                .capture_local_native_action_request(LocalNativeActionInputRequest::new(
                    request.action_id,
                    request.root,
                    request.relative_path,
                )),
            |request| {
                let object = bridge_state::next_object_handle();
                client.native_action_requests.insert(object, request);
                Ok(success(Vec::new(), Vec::new(), object))
            },
        )
    })
}

pub(crate) fn invoke_captured_native_action(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::CapturedActionRequest = crate::decode(request)?;
    bridge_state::with_client_mut(client, |client| {
        let request = client
            .native_action_requests
            .remove(&request.handle)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("captured native-action request", request.handle)
            })?;
        api(client.api.invoke_native_action(request), |execution| {
            Ok(success(Vec::new(), execution.canonical_bytes().to_vec(), 0))
        })
    })
}

pub(crate) fn inspect_local_repository_status(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalRepositoryRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let compilation = client
            .local_compilations
            .get(&request.compilation)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("local compilation", request.compilation)
            })?;
        api(
            client
                .api
                .inspect_local_repository_status(compilation, request.repository_id),
            |status| {
                let drift = status
                    .drift()
                    .iter()
                    .map(|reason| match reason {
                        LocalMaterializationDrift::Surface {
                            relative_path,
                            action,
                        } => LocalDriftMetadata::Surface {
                            relative_path,
                            action: materialization_action(*action),
                        },
                        LocalMaterializationDrift::Ownership { change } => {
                            LocalDriftMetadata::Ownership {
                                change: ownership_change(*change),
                            }
                        }
                        LocalMaterializationDrift::MachineLocal {
                            kind,
                            relative_path,
                            action,
                        } => LocalDriftMetadata::MachineLocal {
                            effect: machine_effect_kind(*kind),
                            relative_path,
                            action: materialization_action(*action),
                        },
                    })
                    .collect();
                json_response(&LocalRepositoryStatusMetadata {
                    repository_id: status.repository_id(),
                    clean: status.is_clean(),
                    drift,
                })
            },
        )
    })
}

pub(crate) fn plan_local_one_shot(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalPlanningRequest = crate::decode(request)?;
    bridge_state::with_client(client, |state| {
        let compilation = state
            .local_compilations
            .get(&request.compilation)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("local compilation", request.compilation)
            })?;
        api(
            state.api.plan_local_one_shot(
                compilation,
                LocalOneShotPlanRequest::new(request.repository_id, request.kind.into()),
            ),
            |plan| {
                let counts = plan.counts();
                let metadata = encode(&LocalPlanMetadata {
                    kind: plan_kind(plan.document().kind()),
                    repository_id: plan.document().repository_id(),
                    baseline_count: counts.baseline(),
                    operation_count: counts.operations(),
                    total_operation_count: counts.total_operations(),
                    create_count: counts.create(),
                    adopt_count: counts.adopt(),
                    update_count: counts.update(),
                    unchanged_count: counts.unchanged(),
                    delete_count: counts.delete(),
                    splice_count: counts.splice(),
                    deferred_effect_count: counts.deferred_effects(),
                    machine_local_effect_count: counts.machine_local_effects(),
                })?;
                let payload = plan.document().canonical_bytes().to_vec();
                let object = bridge_state::next_object_handle();
                let mut response = success(metadata, payload, object);
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::LocalPlan {
                        handle: object,
                        plan,
                    },
                }));
                Ok(response)
            },
        )
    })
}

pub(crate) fn publish_local_one_shot_plan(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalPlanPublicationRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let plan = client
            .local_plans
            .get(&request.plan)
            .ok_or_else(|| BridgeFailure::invalid_handle("local plan", request.plan))?;
        api(
            client
                .api
                .publish_local_one_shot_plan(plan, request.output_directory),
            |publication| {
                json_response(&LocalPlanPublicationMetadata {
                    repository_id: publication.repository_id(),
                    relative_target: publication.relative_target(),
                    display_target: publication.display_target(),
                })
            },
        )
    })
}

pub(crate) fn admit_local_materialization_run(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalRunAdmissionRequest = crate::decode(request)?;
    bridge_state::with_client(client, |state| {
        let compilation = state
            .local_compilations
            .get(&request.compilation)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("local compilation", request.compilation)
            })?;
        api(
            state.api.admit_local_materialization_run(
                compilation,
                LocalMaterializationAdmissionRequest::new(
                    request.kind.into(),
                    request.saved_plan_directory,
                    request.evidence_directory,
                ),
            ),
            |run| {
                let metadata = encode(&LocalRunAdmissionMetadata {
                    kind: plan_kind(run.kind()),
                    targets: run
                        .targets()
                        .iter()
                        .map(|target| LocalRunTargetAdmissionMetadata {
                            repository_id: target.repository_id(),
                            plan_digest: target.plan_digest(),
                        })
                        .collect(),
                })?;
                let object = bridge_state::next_object_handle();
                let mut response = success(metadata, Vec::new(), object);
                response.pending_values = Some(Box::new(bridge_state::PendingResponse {
                    client,
                    values: bridge_state::PendingResponseValues::LocalRun {
                        handle: object,
                        run,
                    },
                }));
                Ok(response)
            },
        )
    })
}

pub(crate) fn settle_local_materialization_run(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalRunRequest = crate::decode(request)?;
    bridge_state::with_client_mut(client, |client| {
        let run = client
            .local_runs
            .get_mut(&request.run)
            .ok_or_else(|| BridgeFailure::invalid_handle("local run", request.run))?;
        api(client.api.settle_local_materialization_run(run), |result| {
            let targets = result
                .targets()
                .iter()
                .map(|target| {
                    let failure = target
                        .failure()
                        .map(|failure| base64(failure.canonical_bytes()));
                    Ok(LocalRunTargetMetadata {
                        repository_id: target.repository_id(),
                        outcome: target_outcome(target.outcome()),
                        changed_surface_count: target.changed_surface_count(),
                        deferred_effect_count: target.deferred_effect_count(),
                        local_git_exclude_changed: target.local_git_exclude_changed(),
                        failure,
                    })
                })
                .collect::<Result<Vec<_>, BridgeFailure>>()?;
            Ok(success(
                encode(&LocalRunResultMetadata {
                    kind: plan_kind(result.kind()),
                    outcome: run_outcome(result.outcome()),
                    target_count: result.target_count(),
                    succeeded_target_count: result.succeeded_target_count(),
                    targets,
                })?,
                result.canonical_bytes().to_vec(),
                0,
            ))
        })
    })
}

pub(crate) fn inspect_local_materialization_reconciliation(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::LocalRunRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let run = client
            .local_runs
            .get(&request.run)
            .ok_or_else(|| BridgeFailure::invalid_handle("local run", request.run))?;
        api(
            client.api.inspect_local_materialization_reconciliation(run),
            |observation| {
                json_response(&LocalReconciliationMetadata {
                    repository_id: observation.repository_id(),
                    plan_digest: observation.plan_digest(),
                    receipt: receipt_observation(observation.receipt()),
                    ownership: ownership_observation(observation.ownership()),
                })
            },
        )
    })
}

fn compiled_interface_identity_response(
    identity: pale_sdk::CompiledInterfaceIdentity,
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    Ok(success(
        encode(&CompiledInterfaceIdentityMetadata {
            version: identity.version(),
            protocol: identity.protocol(),
            os: identity.os(),
            arch: identity.arch(),
            durable_publication: durable_publication(identity.durable_publication()),
        })?,
        identity.canonical_bytes().to_vec(),
        0,
    ))
}

pub(crate) fn compiled_interface_identity(
    client: u64,
    _request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    bridge_state::with_client(client, |client| {
        api(
            client.api.compiled_interface_identity(),
            compiled_interface_identity_response,
        )
    })
}

pub(crate) fn verify_current_executable_binding(
    client: u64,
    request: &[u8],
) -> Result<crate::bridge_state::Response, BridgeFailure> {
    let request: wire::CurrentExecutableBindingVerificationRequest = crate::decode(request)?;
    bridge_state::with_client(client, |client| {
        let compilation = client
            .local_compilations
            .get(&request.compilation)
            .ok_or_else(|| {
                BridgeFailure::invalid_handle("local compilation", request.compilation)
            })?;
        api(
            client.api.verify_current_executable_binding(compilation),
            compiled_interface_identity_response,
        )
    })
}
