//! Focused inventory of Rust-owned initial-v1 schema identity projections.

use std::collections::BTreeMap;

use pale_core::engine_wire::{
    DecisionEvidenceLedgerV1, DeferredSurfaceManifestV1, ExpiringRecordEnvelopeV1,
    OperationDenialV1, SessionRuntimeStateV1,
};
use pale_core::records::{
    ActionContract, ActionExecution, ActionInput, ActionOutput, ActionReceipt, ActionRequest,
    ActionResult, ActivationFenceWireV1, ActiveGenerationHeadWireV1, AdapterFailureWireV1,
    AdvertisedGenerationHeadWireV1, AgentSurfacePackageV1, AgentSurfacePlanV1,
    CapabilityBindingsV1, CapabilityResultWireV1, CapabilityRevocationCheckpointWireV1,
    CapabilitySessionGrantRevocationWireV1, CapabilitySessionGrantWireV1,
    CommonRequestEnvelopeWireV1, ConnectorCatalog, ConnectorFile, ControlStateCheckpointWireV1,
    DeferredEffect, DoctrineCatalog, DoctrineFile, EffectAdmissionWireV1,
    EffectConflictClaimWireV1, EffectDispatchIntentWireV1, EffectReceiptWireV1,
    EffectiveProjectProfileV1, EventAcknowledgementWireV1, EventReferenceWireV1,
    ExecutionOwnerFenceWireV1, GenealogyEdgeWireV1, GeneratedAgentSurfaceManifestV1,
    GoverningOrderFile, GoverningOrderV1, HarnessProjectionManifestV1, HookContextV1,
    InteractionOccurrenceWireV1, LifecycleEvent, LocalRouteBindingWireV1, MachineLocalRootIdentity,
    MaterializationLineageV1, MaterializationPlanV1, MaterializationReceiptV1,
    MaterializationStateV1, NativeActionCatalogV1, OfficialReference, OperationCatalog,
    OperationContractCatalogV1, OperationContractV1, OperationLeaseWireV1, OperationRouteBindingV1,
    OperationsFile, PackageMeta, PolicyFamilyV1, PublishedGenerationHeadWireV1,
    ReconciliationCaseWireV1, ReconciliationCustodyWireV1, ReconciliationFrontierWireV1,
    ReconciliationResultWireV1, RepositoryNativeActionCatalogV1, ResolvedProjectInstanceV1,
    RoleCatalog, RolesFile, RunModeCatalog, RunModesFile, RunResultV1, SchemaDigest,
    SealedArtifactRef, SessionAdmissionLeaseWireV1, StackCatalog, StacksFile, SurfaceDigest,
    SurfaceExecutableIdentityV1, SurfaceOwnership, ToolCallMatcher, ToolCatalog, ToolFile,
};
use schemars::JsonSchema;
use serde_json::{json, Value};

type Projection = fn() -> Value;

struct SelfCarriedIdentity {
    producer: &'static str,
    project: Projection,
    schema: &'static str,
    schema_version: Option<u32>,
}

struct NestedVersionIdentity {
    root_schema: &'static str,
    producer: &'static str,
    project: Projection,
}

struct IdentityAbsentRoot {
    root_schema: &'static str,
    project: Projection,
}

fn project<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("serialize generated JSON Schema")
}

fn required_fields<T: JsonSchema>() -> Vec<String> {
    project::<T>()["required"]
        .as_array()
        .expect("portable producer has a required-field set")
        .iter()
        .map(|field| {
            field
                .as_str()
                .expect("required-field identity is a string")
                .to_owned()
        })
        .collect()
}

#[test]
fn resident_wire_schema_identity_is_structurally_closed() {
    let cases: [(&str, Projection, &str, &str, Option<u32>); 27] = [
        (
            "AdapterFailureWireV1",
            project::<AdapterFailureWireV1>,
            "schema_id",
            "pale/adapter-failure.v1",
            None,
        ),
        (
            "CapabilityResultWireV1",
            project::<CapabilityResultWireV1>,
            "schema_id",
            "pale/capability-result.v1",
            None,
        ),
        (
            "CommonRequestEnvelopeWireV1",
            project::<CommonRequestEnvelopeWireV1>,
            "envelope_schema_id",
            "pale/common-request-envelope.v1",
            None,
        ),
        (
            "ActivationFenceWireV1",
            project::<ActivationFenceWireV1>,
            "schema",
            "pale/activation-fence.v1",
            Some(1),
        ),
        (
            "ActiveGenerationHeadWireV1",
            project::<ActiveGenerationHeadWireV1>,
            "schema",
            "pale/active-generation-head.v1",
            Some(1),
        ),
        (
            "AdvertisedGenerationHeadWireV1",
            project::<AdvertisedGenerationHeadWireV1>,
            "schema",
            "pale/advertised-generation-head.v1",
            Some(1),
        ),
        (
            "OperationLeaseWireV1",
            project::<OperationLeaseWireV1>,
            "schema",
            "pale/operation-lease.v1",
            Some(1),
        ),
        (
            "EventReferenceWireV1",
            project::<EventReferenceWireV1>,
            "schema_id",
            "pale/event-reference.v1",
            None,
        ),
        (
            "EventAcknowledgementWireV1",
            project::<EventAcknowledgementWireV1>,
            "schema_id",
            "pale/event-acknowledgement.v1",
            None,
        ),
        (
            "CapabilitySessionGrantWireV1",
            project::<CapabilitySessionGrantWireV1>,
            "schema_id",
            "pale/capability-session-grant.v1",
            None,
        ),
        (
            "CapabilitySessionGrantRevocationWireV1",
            project::<CapabilitySessionGrantRevocationWireV1>,
            "schema_id",
            "pale/capability-session-grant-revocation.v1",
            None,
        ),
        (
            "CapabilityRevocationCheckpointWireV1",
            project::<CapabilityRevocationCheckpointWireV1>,
            "schema_id",
            "pale/capability-revocation-checkpoint.v1",
            None,
        ),
        (
            "LocalRouteBindingWireV1",
            project::<LocalRouteBindingWireV1>,
            "schema_id",
            "pale/local-route-binding.v1",
            None,
        ),
        (
            "ExecutionOwnerFenceWireV1",
            project::<ExecutionOwnerFenceWireV1>,
            "schema_id",
            "pale/execution-owner-fence.v1",
            None,
        ),
        (
            "InteractionOccurrenceWireV1",
            project::<InteractionOccurrenceWireV1>,
            "schema_id",
            "pale/interaction-occurrence.v1",
            None,
        ),
        (
            "GenealogyEdgeWireV1",
            project::<GenealogyEdgeWireV1>,
            "schema_id",
            "pale/genealogy-edge.v1",
            None,
        ),
        (
            "ReconciliationFrontierWireV1",
            project::<ReconciliationFrontierWireV1>,
            "schema_id",
            "pale/reconciliation-frontier.v1",
            None,
        ),
        (
            "PublishedGenerationHeadWireV1",
            project::<PublishedGenerationHeadWireV1>,
            "schema",
            "pale/published-generation-head.v1",
            Some(1),
        ),
        (
            "SessionAdmissionLeaseWireV1",
            project::<SessionAdmissionLeaseWireV1>,
            "schema_id",
            "pale/session-admission-lease.v1",
            None,
        ),
        (
            "ControlStateCheckpointWireV1",
            project::<ControlStateCheckpointWireV1>,
            "schema_id",
            "pale/control-state-checkpoint.v1",
            None,
        ),
        (
            "EffectAdmissionWireV1",
            project::<EffectAdmissionWireV1>,
            "schema_id",
            "pale/effect-admission.v1",
            None,
        ),
        (
            "EffectConflictClaimWireV1",
            project::<EffectConflictClaimWireV1>,
            "schema_id",
            "pale/effect-conflict-claim.v1",
            None,
        ),
        (
            "EffectDispatchIntentWireV1",
            project::<EffectDispatchIntentWireV1>,
            "schema_id",
            "pale/effect-dispatch-intent.v1",
            None,
        ),
        (
            "EffectReceiptWireV1",
            project::<EffectReceiptWireV1>,
            "schema_id",
            "pale/effect-receipt.v1",
            None,
        ),
        (
            "ReconciliationCaseWireV1",
            project::<ReconciliationCaseWireV1>,
            "schema_id",
            "pale/reconciliation-case.v1",
            None,
        ),
        (
            "ReconciliationCustodyWireV1",
            project::<ReconciliationCustodyWireV1>,
            "schema_id",
            "pale/reconciliation-custody.v1",
            None,
        ),
        (
            "ReconciliationResultWireV1",
            project::<ReconciliationResultWireV1>,
            "schema_id",
            "pale/reconciliation-result.v1",
            None,
        ),
    ];

    for (producer, project, field, schema, version) in cases {
        let document = project();
        assert_eq!(
            document.pointer(&format!("/properties/{field}/const")),
            Some(&json!(schema)),
            "{producer} must close its exact schema identity"
        );
        match version {
            Some(version) => assert_eq!(
                document.pointer("/properties/schema_version/const"),
                Some(&json!(version)),
                "{producer} must close its explicit initial-v1 major"
            ),
            None => assert!(
                document.pointer("/properties/schema_version").is_none(),
                "{producer} must not invent a second version field"
            ),
        }
    }
}

#[test]
fn current_rust_first_producers_require_their_self_digest() {
    for (producer, field, required) in [
        (
            "MaterializationStateV1",
            "state_digest",
            required_fields::<MaterializationStateV1>(),
        ),
        (
            "MaterializationLineageV1",
            "lineage_digest",
            required_fields::<MaterializationLineageV1>(),
        ),
        (
            "GeneratedAgentSurfaceManifestV1",
            "manifest_digest",
            required_fields::<GeneratedAgentSurfaceManifestV1>(),
        ),
        (
            "AgentSurfacePlanV1",
            "plan_digest",
            required_fields::<AgentSurfacePlanV1>(),
        ),
        (
            "MaterializationPlanV1",
            "plan_digest",
            required_fields::<MaterializationPlanV1>(),
        ),
        (
            "MaterializationReceiptV1",
            "receipt_digest",
            required_fields::<MaterializationReceiptV1>(),
        ),
        (
            "RunResultV1",
            "run_digest",
            required_fields::<RunResultV1>(),
        ),
        (
            "EffectiveProjectProfileV1",
            "profile_digest",
            required_fields::<EffectiveProjectProfileV1>(),
        ),
        (
            "ResolvedProjectInstanceV1",
            "instance_digest",
            required_fields::<ResolvedProjectInstanceV1>(),
        ),
        (
            "OperationContractV1",
            "contract_digest",
            required_fields::<OperationContractV1>(),
        ),
        (
            "OperationContractCatalogV1",
            "catalog_digest",
            required_fields::<OperationContractCatalogV1>(),
        ),
        (
            "OperationRouteBindingV1",
            "binding_digest",
            required_fields::<OperationRouteBindingV1>(),
        ),
    ] {
        assert!(
            required.iter().any(|candidate| candidate == field),
            "{producer} must require {field} in its generated wire contract"
        );
    }
}

#[test]
fn self_carried_producers_project_exact_initial_v1_constants() {
    let cases = [
        SelfCarriedIdentity {
            producer: "DecisionEvidenceLedgerV1",
            project: project::<DecisionEvidenceLedgerV1>,
            schema: "pale/decision-evidence-ledger.v1",
            schema_version: None,
        },
        SelfCarriedIdentity {
            producer: "OperationDenialV1",
            project: project::<OperationDenialV1>,
            schema: "pale/operation-denial.v1",
            schema_version: None,
        },
        SelfCarriedIdentity {
            producer: "SessionRuntimeStateV1",
            project: project::<SessionRuntimeStateV1>,
            schema: "pale.session-runtime-state.v1",
            schema_version: None,
        },
        SelfCarriedIdentity {
            producer: "DeferredSurfaceManifestV1",
            project: project::<DeferredSurfaceManifestV1>,
            schema: "pale.deferred-surface-manifest.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "ExpiringRecordEnvelopeV1",
            project: project::<ExpiringRecordEnvelopeV1<BTreeMap<String, Value>>>,
            schema: "pale.expiring-record-envelope.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "ActionContract",
            project: project::<ActionContract>,
            schema: "pale/action-contract.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "NativeActionCatalogV1",
            project: project::<NativeActionCatalogV1>,
            schema: "pale/native-action-catalog.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "ActionRequest",
            project: project::<ActionRequest>,
            schema: "pale/action-request.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "ActionResult",
            project: project::<ActionResult>,
            schema: "pale/action-result.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "ActionReceipt",
            project: project::<ActionReceipt>,
            schema: "pale/action-receipt.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "ActionExecution",
            project: project::<ActionExecution>,
            schema: "pale/action-execution.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "RepositoryNativeActionCatalogV1",
            project: project::<RepositoryNativeActionCatalogV1>,
            schema: "pale/repository-native-action-catalog.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "MaterializationStateV1",
            project: project::<MaterializationStateV1>,
            schema: "pale.agent-surface.materialization-state.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "MaterializationLineageV1",
            project: project::<MaterializationLineageV1>,
            schema: "pale.agent-surface.materialization-lineage.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "GeneratedAgentSurfaceManifestV1",
            project: project::<GeneratedAgentSurfaceManifestV1>,
            schema: "pale.agent-surface.generated-manifest.v1",
            schema_version: None,
        },
        SelfCarriedIdentity {
            producer: "AgentSurfacePlanV1",
            project: project::<AgentSurfacePlanV1>,
            schema: "pale.agent-surface-plan.v1",
            schema_version: None,
        },
        SelfCarriedIdentity {
            producer: "MaterializationPlanV1",
            project: project::<MaterializationPlanV1>,
            schema: "pale.materialization-plan.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "EffectiveProjectProfileV1",
            project: project::<EffectiveProjectProfileV1>,
            schema: "pale.agent-surface.effective-profile.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "ResolvedProjectInstanceV1",
            project: project::<ResolvedProjectInstanceV1>,
            schema: "pale.agent-surface.resolved-instance.v1",
            schema_version: None,
        },
        SelfCarriedIdentity {
            producer: "HarnessProjectionManifestV1",
            project: project::<HarnessProjectionManifestV1>,
            schema: "pale.agent-surface.harness-projection.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "MaterializationReceiptV1",
            project: project::<MaterializationReceiptV1>,
            schema: "pale.agent-surface.receipt.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "RunResultV1",
            project: project::<RunResultV1>,
            schema: "pale.agent-surface.run-result.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "HookContextV1",
            project: project::<HookContextV1>,
            schema: "pale.agent-surface.hook-context.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "SurfaceExecutableIdentityV1",
            project: project::<SurfaceExecutableIdentityV1>,
            schema: "pale/surface-executable-identity.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "CapabilityBindingsV1",
            project: project::<CapabilityBindingsV1>,
            schema: "pale.agent-surface.capability-bindings.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "OperationContractV1",
            project: project::<OperationContractV1>,
            schema: "pale/operation-contract.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "OperationContractCatalogV1",
            project: project::<OperationContractCatalogV1>,
            schema: "pale/operation-catalog.v1",
            schema_version: Some(1),
        },
        SelfCarriedIdentity {
            producer: "OperationRouteBindingV1",
            project: project::<OperationRouteBindingV1>,
            schema: "pale/operation-route-binding.v1",
            schema_version: Some(1),
        },
    ];

    for case in cases {
        let document = (case.project)();
        assert_eq!(
            document.pointer("/properties/schema/type"),
            Some(&json!("string")),
            "{} must project schema as its wire string",
            case.producer
        );
        assert_eq!(
            document.pointer("/properties/schema/const"),
            Some(&json!(case.schema)),
            "{} must own its exact schema constant",
            case.producer
        );
        match case.schema_version {
            Some(version) => assert_eq!(
                document.pointer("/properties/schema_version/const"),
                Some(&json!(version)),
                "{} must own its exact schema-version constant",
                case.producer
            ),
            None => assert!(
                document.pointer("/properties/schema_version").is_none(),
                "{} must not invent a schema_version member",
                case.producer
            ),
        }
    }
}

#[test]
fn loader_selected_roots_keep_identity_nested_in_their_catalog_table() {
    let nested = [
        NestedVersionIdentity {
            root_schema: "agent-surface-package.v1",
            producer: "PackageMeta",
            project: project::<PackageMeta>,
        },
        NestedVersionIdentity {
            root_schema: "tool.v1",
            producer: "ToolCatalog",
            project: project::<ToolCatalog>,
        },
        NestedVersionIdentity {
            root_schema: "connector.v1",
            producer: "ConnectorCatalog",
            project: project::<ConnectorCatalog>,
        },
        NestedVersionIdentity {
            root_schema: "doctrine-entry.v1",
            producer: "DoctrineCatalog",
            project: project::<DoctrineCatalog>,
        },
        NestedVersionIdentity {
            root_schema: "governing-order.v1",
            producer: "GoverningOrderV1",
            project: project::<GoverningOrderV1>,
        },
        NestedVersionIdentity {
            root_schema: "operation.v1",
            producer: "OperationCatalog",
            project: project::<OperationCatalog>,
        },
        NestedVersionIdentity {
            root_schema: "role.v1",
            producer: "RoleCatalog",
            project: project::<RoleCatalog>,
        },
        NestedVersionIdentity {
            root_schema: "run-mode.v1",
            producer: "RunModeCatalog",
            project: project::<RunModeCatalog>,
        },
        NestedVersionIdentity {
            root_schema: "stack.v1",
            producer: "StackCatalog",
            project: project::<StackCatalog>,
        },
    ];

    for case in nested {
        let document = (case.project)();
        assert_eq!(
            document.pointer("/properties/schema_version/const"),
            Some(&json!(1)),
            "{} nested producer for {} must own initial-v1",
            case.producer,
            case.root_schema
        );
        assert!(
            document.pointer("/properties/schema").is_none(),
            "{} nested producer for {} must not invent a schema member",
            case.producer,
            case.root_schema
        );
    }

    let roots = [
        IdentityAbsentRoot {
            root_schema: "agent-surface-package.v1",
            project: project::<AgentSurfacePackageV1>,
        },
        IdentityAbsentRoot {
            root_schema: "tool.v1",
            project: project::<ToolFile>,
        },
        IdentityAbsentRoot {
            root_schema: "connector.v1",
            project: project::<ConnectorFile>,
        },
        IdentityAbsentRoot {
            root_schema: "doctrine-entry.v1",
            project: project::<DoctrineFile>,
        },
        IdentityAbsentRoot {
            root_schema: "governing-order.v1",
            project: project::<GoverningOrderFile>,
        },
        IdentityAbsentRoot {
            root_schema: "operation.v1",
            project: project::<OperationsFile>,
        },
        IdentityAbsentRoot {
            root_schema: "role.v1",
            project: project::<RolesFile>,
        },
        IdentityAbsentRoot {
            root_schema: "run-mode.v1",
            project: project::<RunModesFile>,
        },
        IdentityAbsentRoot {
            root_schema: "stack.v1",
            project: project::<StacksFile>,
        },
    ];

    for root in roots {
        let document = (root.project)();
        assert!(
            document.pointer("/properties/schema").is_none(),
            "{} loader root must not invent a schema member",
            root.root_schema
        );
        assert!(
            document.pointer("/properties/schema_version").is_none(),
            "{} loader root must keep schema_version nested",
            root.root_schema
        );
    }
}

#[test]
fn identity_absent_catalog_roots_do_not_gain_record_coordinates() {
    let roots = [
        IdentityAbsentRoot {
            root_schema: "action-input.v1",
            project: project::<ActionInput>,
        },
        IdentityAbsentRoot {
            root_schema: "action-output.v1",
            project: project::<ActionOutput>,
        },
        IdentityAbsentRoot {
            root_schema: "deferred-effect.v1",
            project: project::<DeferredEffect>,
        },
        IdentityAbsentRoot {
            root_schema: "lifecycle-event.v1",
            project: project::<LifecycleEvent>,
        },
        IdentityAbsentRoot {
            root_schema: "machine-local-root-identity.v1",
            project: project::<MachineLocalRootIdentity>,
        },
        IdentityAbsentRoot {
            root_schema: "official-reference.v1",
            project: project::<OfficialReference>,
        },
        IdentityAbsentRoot {
            root_schema: "policy-family.v1",
            project: project::<PolicyFamilyV1>,
        },
        IdentityAbsentRoot {
            root_schema: "sealed-artifact.v1",
            project: project::<SealedArtifactRef>,
        },
        IdentityAbsentRoot {
            root_schema: "sha256-digest.v1",
            project: project::<SchemaDigest>,
        },
        IdentityAbsentRoot {
            root_schema: "surface-digest.v1",
            project: project::<SurfaceDigest>,
        },
        IdentityAbsentRoot {
            root_schema: "surface-ownership.v1",
            project: project::<SurfaceOwnership>,
        },
        IdentityAbsentRoot {
            root_schema: "tool-call-matcher.v1",
            project: project::<ToolCallMatcher>,
        },
    ];

    for root in roots {
        let document = (root.project)();
        assert!(
            document.pointer("/properties/schema").is_none(),
            "{} must not invent a schema member",
            root.root_schema
        );
        assert!(
            document.pointer("/properties/schema_version").is_none(),
            "{} must not invent a schema_version member",
            root.root_schema
        );
    }
}
