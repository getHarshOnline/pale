//! Shared source-backed local operation preparation fixtures.
//! Compiler and contextual admission are real; signing and external authority
//! callbacks are declared test models. No native accumulator result or custody
//! outcome is fabricated here. Tests supply their actual storage/anchor owner.

use super::effect_case_lineage_fixture as request_fixture;
use super::resident::local_operation_start_test_support::prepare_local_start;
use super::resident::*;
use super::resident_fixture::{applied_evidence, compile_live, live_workspace};
use super::session_admission_fixture::{
    with_local_start_session_context_at_store,
    with_local_start_session_context_at_store_and_authorities, LocalStartContext,
    LocalStartExistingAuthorities, LocalStartSessionStore, ProvingAuthority,
};
use pale_core::records::*;
use pale_ports::AuthorityStoreCoordinate;
use std::cell::Cell;

pub(crate) fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

pub(crate) fn schema_digest(schema_id: &str) -> SchemaDigest {
    pale_contracts::record_schema_identity(schema_id)
        .expect("schema is registered")
        .schema_sha256
}

pub(crate) fn artifact_digest(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

pub(crate) fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

pub(crate) fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

pub(crate) fn publication_writer() -> PrincipalResidentWriterFenceV1 {
    let schema =
        pale_contracts::record_schema_identity(PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA_ID)
            .expect("writer-fence schema");
    let preimage = PrincipalResidentWriterFencePreimageV1 {
        schema_id: schema.schema,
        schema_sha256: schema.schema_sha256,
        writer_fence_id: PrincipalResidentWriterFenceIdV1::parse("publication-writer")
            .expect("writer fence"),
        principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        writer_generation: PrincipalResidentWriterGenerationV1::new(1).expect("writer generation"),
        previous_writer_fence_ref: None,
        previous_writer_fence_sha256: None,
        prior_terminal_receipt_ref: None,
        prior_terminal_receipt_sha256: None,
        takeover_receipt_ref: None,
        takeover_receipt_sha256: None,
        primary_node_id: ResidentNodeIdV1::parse("publication-node").expect("node"),
        process_instance_id: ResidentProcessInstanceIdV1::parse("publication-process")
            .expect("process"),
        boot_incarnation_id: ResidentBootIncarnationIdV1::parse("publication-boot")
            .expect("boot incarnation"),
        trust_store_id: ControlStoreIdV1::parse("trust-store").expect("trust store"),
        trust_store_incarnation_id: ControlIncarnationIdV1::parse("trust-incarnation")
            .expect("trust-store incarnation"),
        opened_deployment_store_heads_root_sha256: ArtifactDigest::hash_bytes(
            b"publication-opened-deployment-heads",
        ),
        principal_session_floor_epoch: PrincipalSessionFloorEpochV1::new(1).expect("session floor"),
        state: PrincipalResidentWriterFenceStateV1::Active,
        acquired_at: UnixTimeV1::new(50).expect("acquisition time"),
        retired_at: None,
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("principal-key-1").expect("key"),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("key epoch"),
    };
    let digest =
        PrincipalResidentWriterFenceDigest::hash_canonical(&preimage).expect("writer-fence digest");
    let mut value = serde_json::to_value(&preimage).expect("writer fence serializes");
    value
        .as_object_mut()
        .expect("writer fence is an object")
        .insert(
            "canonical_principal_resident_writer_fence_sha256".into(),
            serde_json::to_value(digest).expect("digest serializes"),
        );
    value
        .as_object_mut()
        .expect("writer fence is an object")
        .insert(
            "signature".into(),
            serde_json::to_value(signature()).expect("signature serializes"),
        );
    let principal = ResidentPrincipalIdV1::parse("principal-1").expect("principal");
    let key = ResidentKeyIdV1::parse("principal-key-1").expect("key");
    PrincipalResidentWriterFenceV1::admit_initial_active_wire(
        serde_json::from_value(value).expect("writer-fence wire"),
        &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
        &preimage.schema_sha256,
        &PrincipalKeyAdmissionContextV1::new(
            &principal,
            &key,
            PrincipalKeyEpochV1::new(1).expect("key epoch"),
        ),
        |_, _, _, _| Ok(()),
    )
    .expect("writer fence admits")
}

fn publication_actor(suffix: &str) -> OwnerRecordCoordinateV1 {
    OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse("pale-test-owner").expect("owner system"),
        record_ref: record_ref(&format!("actor/{suffix}")),
        record_schema_id: SchemaId::parse("test/actor.v1").expect("actor schema"),
        record_schema_sha256: schema_digest(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID),
        record_content_sha256: artifact_digest('9'),
        native_revision: NativeRevisionV1 {
            native_revision_owner: NativeRevisionOwnerV1::parse("pale-test-owner")
                .expect("revision owner"),
            native_revision_token: NativeRevisionTokenV1::parse(format!("actor-revision-{suffix}"))
                .expect("revision token"),
        },
    }
}

// Shared signed-observation fixture. Authority/evidence callbacks are declared
// test models; this helper does not assert actual native effect or custody.
#[allow(clippy::too_many_arguments)]
pub(crate) fn issue_observation(
    authority: &SigningAuthorityBindingV1,
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    schema_sha256: SchemaDigest,
    revision: OperationObservationRevisionV1,
    predecessor: Option<(&ResidentRecordRefV1, &OperationObservationV1)>,
    state: OperationObservationStateV1,
    observed_at: u64,
) -> OperationObservationV1 {
    let runtime_authority = authority
        .integrated_runtime_authority()
        .expect("test observations use integrated runtime authority");
    let predecessor_ref = predecessor.map(|(reference, _)| reference.clone());
    let predecessor_sha256 =
        predecessor.map(|(_, record)| record.canonical_operation_observation_sha256().clone());
    OperationObservationV1::issue_contextual(
        OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                .expect("observation schema"),
            schema_sha256: schema_sha256.clone(),
            operation_handle_id: handle.operation_handle_id().clone(),
            operation_handle_ref: handle_ref.clone(),
            operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
            revision,
            predecessor_ref,
            predecessor_sha256,
            state,
            observed_at: UnixTimeV1::new(observed_at).expect("observation time"),
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: authority.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(authority.signer_key_id())
                .expect("admitted signer key"),
            issuer_key_epoch: *authority.signer_key_epoch(),
        },
        &OperationObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema_sha256,
                signing_authority: authority,
            },
            handle_ref,
            handle,
            predecessor,
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(signature()),
    )
    .expect("observation admits before signing")
}

pub(crate) fn checkpoint_preimage(
    sequence: u64,
    predecessor: Option<(ResidentRecordRefV1, ControlStateCheckpointDigest)>,
) -> ControlStateCheckpointPreimageV1 {
    checkpoint_preimage_at_store(sequence, predecessor, &store_coordinate())
}

pub(crate) fn checkpoint_preimage_at_store(
    sequence: u64,
    predecessor: Option<(ResidentRecordRefV1, ControlStateCheckpointDigest)>,
    store: &AuthorityStoreCoordinate,
) -> ControlStateCheckpointPreimageV1 {
    ControlStateCheckpointPreimageV1 {
        schema_id: SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA).expect("checkpoint schema"),
        schema_sha256: schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
        control_checkpoint_id: ControlCheckpointIdV1::parse(format!(
            "operation-checkpoint-{sequence}"
        ))
        .expect("checkpoint identity"),
        control_store_id: store.store_id().clone(),
        control_incarnation_id: store.incarnation_id().clone(),
        checkpoint_domain_id: ControlCheckpointDomainIdV1::parse("operation:principal-1")
            .expect("checkpoint domain"),
        domain_epoch: ControlDomainEpochV1::new(1).expect("epoch"),
        domain_sequence: ControlDomainSequenceV1::new(sequence).expect("sequence"),
        previous_control_checkpoint_ref: predecessor
            .as_ref()
            .map(|(reference, _)| reference.clone()),
        previous_control_checkpoint_sha256: predecessor.map(|(_, digest)| digest),
        checkpoint_profile_ref: record_ref("operation/checkpoint-profile"),
        checkpoint_profile_sha256: CheckpointProfileDigest::parse(sha('b'))
            .expect("profile digest"),
        control_schema_version: 1,
        committed_transaction_sequence: ControlTransactionSequenceV1::new(sequence)
            .expect("transaction sequence"),
        state_root_sha256: artifact_digest(if sequence == 1 { 'c' } else { 'd' }),
        leaf_count: ControlLeafCountV1::new(2).expect("leaf count"),
        changed_leaf_keys: vec![ChangedControlLeafKeyV1 {
            record_class: ControlRecordClassV1::parse("operation-observation")
                .expect("record class"),
            scope_key_sha256: artifact_digest('e'),
            logical_key: ControlLogicalKeyV1::parse("operation-1").expect("logical key"),
        }],
        unresolved_custody_root_sha256: artifact_digest('f'),
        issued_at: UnixTimeV1::new(40 + sequence).expect("checkpoint time"),
        checkpoint_signing_authority_ref: record_ref("checkpoint-authority"),
        checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest::parse(sha('1'))
            .expect("authority digest"),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("key epoch"),
    }
}

pub(crate) fn checkpoint_authority(
    value: &ControlStateCheckpointPreimageV1,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: &value.checkpoint_signing_authority_ref,
        authority_sha256: &value.checkpoint_signing_authority_sha256,
        principal_id: &value.issuer_principal_id,
        control_store_id: &value.control_store_id,
        control_incarnation_id: &value.control_incarnation_id,
        checkpoint_domain_id: &value.checkpoint_domain_id,
        checkpoint_profile_ref: &value.checkpoint_profile_ref,
        checkpoint_profile_sha256: &value.checkpoint_profile_sha256,
        subject_key_id: &value.issuer_key_id,
        subject_key_epoch: value.issuer_key_epoch,
    }
}

pub(crate) fn store_coordinate() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation"),
    )
}

pub(crate) fn operation_lease_ref() -> ResidentRecordRefV1 {
    record_ref("start-lease")
}

pub(crate) fn profile() -> super::resident::SupportedOperationCheckpointProfile {
    let digest = super::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
    super::resident::SupportedOperationCheckpointProfile::admit(
        record_ref("operation/checkpoint-profile"),
        &digest,
        super::resident::SupportedOperationCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap()
}

pub(crate) fn operation_id() -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.engine").expect("package"),
        resource_id: PackageResourceIdV1::parse("review").expect("operation"),
    }
}

pub(crate) fn source_generation_inputs_with(
    change_operation: impl FnOnce(&mut OnDemandOperation),
    change_private: impl FnOnce(&mut CapabilitiesFile),
) -> (
    super::compile::Compilation,
    Vec<AppliedRepositoryMaterialization>,
    EffectiveProjectProfileV1,
) {
    source_generation_input_copies_with(change_operation, change_private, 1)
        .pop()
        .expect("one requested source generation")
}

/// Compile several byte-identical generations from one immutable source tree.
///
/// Some resident tests must move one admitted generation into an application
/// while retaining another as the operation fixture's source. Recompiling the
/// same source tree preserves the canonical digest without making the resident
/// generation aggregate cloneable.
pub(crate) fn source_generation_input_copies_with(
    change_operation: impl FnOnce(&mut OnDemandOperation),
    change_private: impl FnOnce(&mut CapabilitiesFile),
    copies: usize,
) -> Vec<(
    super::compile::Compilation,
    Vec<AppliedRepositoryMaterialization>,
    EffectiveProjectProfileV1,
)> {
    assert!(copies > 0, "source-generation copy count must be positive");
    let (workspace, probe) = live_workspace();
    let package = workspace.root.path().join("distribution/packages/engine");
    let manifest_path = package.join("package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest_path).expect("existing role Package"))
            .expect("Package manifest");
    let owner = manifest.get_mut("package").expect("Package table");
    let sources = owner
        .get_mut("sources")
        .and_then(toml::Value::as_array_mut)
        .expect("source inventory");
    for source in [
        "{path='operations.toml', role='declaration'}",
        "{path='policy.toml', role='declaration'}",
        "{path='review.md', role='body'}",
    ] {
        let entry: toml::Table = format!("entry={source}")
            .parse()
            .expect("test source entry");
        sources.push(entry["entry"].clone());
    }
    let resources = owner
        .get_mut("resources")
        .and_then(toml::Value::as_array_mut)
        .expect("resource inventory");
    for resource in [
        "{id='review', kind='operation', definition_source='operations.toml', exposure='exported'}",
        "{id='review-policy', kind='capability_policy', definition_source='policy.toml', exposure='exported'}",
    ] {
        let entry: toml::Table = format!("entry={resource}")
            .parse()
            .expect("test resource entry");
        resources.push(entry["entry"].clone());
    }
    std::fs::write(
        manifest_path,
        toml::to_string(&manifest).expect("manifest serialization"),
    )
    .expect("write selected sources");
    let mut operation = OnDemandOperation {
        id: operation_id().resource_id,
        description: "Inspect one named bounded capability.".into(),
        doctrine: pale_core::paths::OwnedConfinedRelativePath::parse("review.md").expect("body"),
        extends: Vec::new(),
        doctrine_refs: Vec::new(),
        roles: Vec::new(),
        outputs: Vec::new(),
        capability_requirements: ["first", "second"]
            .into_iter()
            .map(|id| OperationCapabilityRequirement {
                id: CapabilityRequirementIdV1::parse(id).expect("requirement"),
                description: "Read the selected evidence.".into(),
                capability: CapabilityIdV1::parse("read").expect("capability"),
                recipe: None,
                effect_class: EffectClass::Compute,
                policy: CapabilityRequirementPolicy::RequiredBlock,
                failure_posture: FailurePosture::FailClosed,
            })
            .collect(),
        capability_policy: Some(PackageResourceLinkV1::Local {
            resource_id: PackageResourceIdV1::parse("review-policy").expect("policy"),
        }),
    };
    change_operation(&mut operation);
    let mut permitted_capabilities: Vec<_> = operation
        .capability_requirements
        .iter()
        .map(|requirement| requirement.capability.clone())
        .collect();
    permitted_capabilities.sort();
    permitted_capabilities.dedup();
    let mut permitted_effects: Vec<_> = operation
        .capability_requirements
        .iter()
        .map(|requirement| requirement.effect_class)
        .collect();
    permitted_effects.sort();
    permitted_effects.dedup();
    let operations =
        serde_json::json!({"operations": {"schema_version": 1, "operation": [operation]}});
    std::fs::write(
        package.join("operations.toml"),
        toml::to_string(&operations).expect("operations source"),
    )
    .expect("write operations");
    std::fs::write(
        package.join("review.md"),
        "# Review\n\nInspect admitted evidence only.\n",
    )
    .expect("write body");
    std::fs::write(
        package.join("policy.toml"),
        toml::to_string(&CapabilityPolicy {
            permitted_capabilities,
            forbidden_capabilities: Vec::new(),
            permitted_effects,
            forbidden_effects: Vec::new(),
        })
        .expect("policy source"),
    )
    .expect("write policy");
    let capabilities_path = workspace.root.path().join("workspace/capabilities.toml");
    let mut capabilities: CapabilitiesFile =
        toml::from_str(&std::fs::read_to_string(&capabilities_path).expect("private source"))
            .expect("private capabilities");
    capabilities
        .semantic
        .operations
        .push(PackageResourceSelectorV1::Exact {
            resource: operation_id(),
        });
    capabilities
        .operation_bindings
        .push(OperationBindingEntryV1 {
            operation: operation_id(),
            binding: OperationBinding {
                selection: OperationSelection::ModelOrUser {
                    aliases: Vec::new(),
                },
            },
        });
    capabilities.capability_bindings.insert(
        CapabilityIdV1::parse("read").expect("capability"),
        CapabilityBinding {
            effect_class: EffectClass::Compute,
            approval: Approval::None,
            tool_contract: None,
            implementation: CapabilityProviderBinding::NativeRust {
                target: PATH_INVENTORY_ACTION_ID.into(),
            },
            note: None,
        },
    );
    change_private(&mut capabilities);
    std::fs::write(
        capabilities_path,
        toml::to_string(&capabilities).expect("private source"),
    )
    .expect("write private selection");
    workspace.refresh_distribution_lock();
    (0..copies)
        .map(|_| {
            let compilation = compile_live(&workspace, &probe);
            let profile = compilation.profile().clone();
            let materialization = applied_evidence(&compilation).into_materialization();
            (compilation, vec![materialization], profile)
        })
        .collect()
}

/// Compile the shared operation fixture with one selected host-provider tool.
pub(crate) fn host_source_generation_inputs(
    tool_name: &str,
) -> (
    super::compile::Compilation,
    Vec<AppliedRepositoryMaterialization>,
    EffectiveProjectProfileV1,
) {
    host_source_generation_input_copies(tool_name, 1)
        .pop()
        .expect("one requested host source generation")
}

/// Compile byte-identical host-provider generations for move-only resident fixtures.
pub(crate) fn host_source_generation_input_copies(
    tool_name: &str,
    copies: usize,
) -> Vec<(
    super::compile::Compilation,
    Vec<AppliedRepositoryMaterialization>,
    EffectiveProjectProfileV1,
)> {
    host_source_generation_input_copies_with(tool_name, copies, |_| {})
}

/// Customize the operation while preserving the shared native provider mapping.
pub(crate) fn host_source_generation_input_copies_with(
    tool_name: &str,
    copies: usize,
    change_operation: impl FnOnce(&mut OnDemandOperation),
) -> Vec<(
    super::compile::Compilation,
    Vec<AppliedRepositoryMaterialization>,
    EffectiveProjectProfileV1,
)> {
    let source =
        super::composition_source_fixtures::workspace_composition_source(&["direct-claude-code"]);
    let binding = &source.declarations.provider_bindings[0];
    let reference = ProviderBindingRefV1 {
        provider_binding_id: binding.id.clone(),
        provider_binding_identity: binding.binding_identity.clone(),
    };
    source_generation_input_copies_with(
        change_operation,
        |private| {
            private
                .capability_bindings
                .get_mut(&CapabilityIdV1::parse("read").unwrap())
                .unwrap()
                .implementation = CapabilityProviderBinding::HostNative {
                provider_tools: vec![ProviderToolBindingV1 {
                    provider_binding: reference,
                    tools: vec![HostToolBinding {
                        name: tool_name.into(),
                        model_name: None,
                        observability: ToolObservability::PreToolObservable,
                        commands: vec![],
                        command_source: None,
                        input_equals: Default::default(),
                    }],
                }],
            };
        },
        copies,
    )
}

#[derive(Clone, Copy)]
pub(crate) struct RecoveryParents<'a> {
    pub(crate) session: &'a super::resident::AdmittedAmbientSession,
    pub(crate) current: &'a super::resident::AmbientSessionAdmissionContext<'a>,
    pub(crate) lease: &'a OperationLeaseV1,
    pub(crate) signing: &'a SigningAuthorityBindingV1,
}

/// Existing current admission after protected provenance recovery. Current
/// frontier assertions remain fixture authority, not authority from stored bytes.
pub(crate) fn assert_original_session_revalidation(
    operation: &IssuedLocalOperation,
    parents: &RecoveryParents<'_>,
) {
    assert_eq!(operation.historical_operation_lease(), parents.lease);
    assert_eq!(
        operation.historical_session_lease(),
        parents.session.lease()
    );
    assert_eq!(
        operation
            .historical_operation_lease()
            .operation_lease_digest(),
        operation.handle().1.operation_lease_sha256()
    );
    operation
        .historical_operation_lease()
        .verify_against_parent(operation.historical_session_lease())
        .unwrap();
    let original = parents.current;
    let context = |now, grant, revocations| AmbientSessionAdmissionContext {
        now,
        grant,
        session_revocations: revocations,
        subject: original.subject,
        current_principal_trust: original.current_principal_trust,
        capability_revocations: original.capability_revocations,
        deployment_binding_ref: original.deployment_binding_ref,
        deployment_binding: original.deployment_binding,
        deployment_revocations: original.deployment_revocations,
        runtime_authority: original.runtime_authority,
        route_use: original.route_use,
        generation: SessionAdmissionLeaseContextV1 {
            active_head: original.generation.active_head,
            compiled_generation: original.generation.compiled_generation,
            effective_profile: original.generation.effective_profile,
            activation_fence: original.generation.activation_fence,
            signed: original.generation.signed,
        },
    };
    let resumed = operation
        .revalidate_session(original, &mut ProvingAuthority)
        .expect("same original basis still current");
    assert_eq!(resumed.lease(), parents.session.lease());
    let lease = operation.historical_operation_lease();
    let narrowed = resumed
        .admit_operation(
            schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
            OperationLeaseRequestV1 {
                operation_lease_id: OperationLeaseIdV1::parse("resumed-session-consumer").unwrap(),
                operation: lease.operation().clone(),
                capabilities: lease.capabilities().to_vec(),
                effects: lease.effects().to_vec(),
                read_set: lease.read_set().to_vec(),
                evidence_obligations: lease.evidence_obligations().to_vec(),
                budget_units: *lease.budget_units(),
                timeout_millis: *lease.timeout_millis(),
                expires_at: lease.expires_at(),
                invocation_correlation: lease.invocation_correlation().clone(),
            },
            original,
            &mut ProvingAuthority,
        )
        .expect("returned admission feeds actual operation narrowing");
    narrowed.verify_against_parent(resumed.lease()).unwrap();
    let expired = context(
        *parents.session.lease().expires_at(),
        original.grant,
        original.session_revocations,
    );
    assert!(
        matches!(
            operation.revalidate_session(&expired, &mut ProvingAuthority),
            Err(AmbientSessionAdmissionError::SessionOutsideValidity)
        ),
        "historical recovery cannot renew expired authority"
    );
    let revoked = SessionRevocationFrontier {
        epoch: PrincipalRevocationEpochV1::new(original.session_revocations.epoch.get() + 1)
            .unwrap(),
        revocation_ref: original.session_revocations.revocation_ref,
        revocation_sha256: original.session_revocations.revocation_sha256,
    };
    let revoked_context = context(original.now, original.grant, &revoked);
    assert!(matches!(
        operation.revalidate_session(&revoked_context, &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::SessionRevocationFrontier
        ))
    ));
    let another_ref = record_ref("grant/independently-valid-other-reference");
    let changed = context(
        original.now,
        Some(CapabilityGrantAuthority {
            grant_ref: &another_ref,
            ..original.grant.unwrap()
        }),
        original.session_revocations,
    );
    let independently_admitted = admit_ambient_session(
        SessionAdmissionLeaseWireV1::from(parents.session.lease()),
        &changed,
        &mut ProvingAuthority,
    )
    .expect("different non-lease basis may admit afresh");
    assert_eq!(independently_admitted.lease(), resumed.lease());
    assert!(
        matches!(
            operation.revalidate_session(&changed, &mut ProvingAuthority),
            Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
        ),
        "existing operation must not adopt a replacement basis"
    );
}

/// Explicit ingress and remaining-authority model selected by each fixture caller.
#[derive(Clone, Copy)]
pub(crate) enum StartAdmission<'a> {
    AcceptedOwnerModel,
    #[allow(
        dead_code,
        reason = "The SQLite recovery fixture retains complete Consult plans."
    )]
    EvidenceConsult {
        build_collection: fn(&OperationHandleV1) -> ConsultationCollectionAdmission,
        limits: ConsultationCollectionLimits,
    },
    #[allow(
        dead_code,
        reason = "Application tests exercise refusal; storage tests also compile this shared fixture."
    )]
    RejectedOwnerModel,
    NativeInput {
        registry: &'a super::actions::NativeActionRegistry,
        reject_owner: bool,
    },
}

/// The schema-correct input for this fixture's compiled path-inventory target.
pub(crate) fn native_request_body(request: &mut CommonRequestEnvelopePreimageV1) {
    let schema = pale_contracts::schema_descriptor("action-input.v1").unwrap();
    request.body.schema_id = SchemaId::parse(schema.schema_id).unwrap();
    request.body.schema_sha256 = SchemaDigest::parse(schema.schema_sha256).unwrap();
    request.body.payload = serde_json::json!({"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs","src/lib.rs"]}});
    request.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&request.body.payload)
            .unwrap()
            .as_bytes(),
    );
}

pub(crate) fn with_start<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_start_context(
        generation,
        profile,
        suffix,
        change_request,
        admission,
        LocalStartContext::Inherited,
        run,
    )
}

#[allow(
    dead_code,
    reason = "shared adapter fixtures do not exercise Consult starts"
)]
pub(crate) fn with_consult_start<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_start_context_class(
        generation,
        profile,
        suffix,
        change_request,
        admission,
        LocalStartContext::Inherited,
        OperationClassV1::Consult,
        false,
        None,
        run,
    )
}

pub(crate) fn with_start_context<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    context_selection: LocalStartContext,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_start_context_at_store(
        generation,
        profile,
        suffix,
        change_request,
        admission,
        context_selection,
        &store_coordinate(),
        run,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn with_start_context_at_store<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    context_selection: LocalStartContext,
    store: &AuthorityStoreCoordinate,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_start_context_class_at_store(
        generation,
        profile,
        suffix,
        change_request,
        admission,
        context_selection,
        OperationClassV1::Invoke,
        store,
        false,
        None,
        |prepared, request, owner_calls, sign_calls, parents| {
            run(prepared, request, owner_calls, sign_calls, parents)
        },
    )
}

/// Prepare one Invoke start against an already admitted deployment authority.
#[allow(clippy::too_many_arguments)]
pub(crate) fn with_start_context_at_store_and_authorities<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    store: &AuthorityStoreCoordinate,
    trust_ref: &ResidentRecordRefV1,
    trust: &PrincipalTrustCheckpointV1,
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    runtime_ref: &ResidentRecordRefV1,
    runtime: &DeploymentRuntimeAuthorityV1,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    let operation = OperationLeaseRequestV1 {
        operation_lease_id: OperationLeaseIdV1::parse("start-lease").expect("lease"),
        operation: operation_id(),
        capabilities: vec![CapabilityIdV1::parse("read").expect("capability")],
        effects: vec![EffectClass::Compute],
        read_set: vec![record_ref("read-a")],
        evidence_obligations: vec![record_ref("evidence-a")],
        budget_units: LeaseBudgetUnitsV1::new(5).expect("budget"),
        timeout_millis: LeaseTimeoutMillisV1::new(500).expect("timeout"),
        expires_at: UnixTimeV1::new(90).expect("expiry"),
        invocation_correlation: InvocationCorrelationV1::Unobserved {
            reason: ResidentReasonV1::parse("not observed by this fixture").expect("reason"),
        },
    };
    with_local_start_session_context_at_store_and_authorities(
        generation,
        profile,
        operation,
        LocalStartContext::Inherited,
        store,
        trust_ref,
        trust,
        binding_ref,
        binding,
        runtime_ref,
        runtime,
        |session, current, lease, signing| {
            let grant = current.grant.expect("fixture grant authority").grant;
            let capability = current
                .capability_revocations
                .expect("fixture capability-revocation checkpoint");
            let seal_current_request = |request: &mut CommonRequestEnvelopePreimageV1| {
                request.session_admission = CommonRequestSessionAdmissionV1 {
                    session_admission_id: session.lease().session_admission_id().clone(),
                    session_admission_lease_digest: session
                        .lease()
                        .session_admission_lease_digest()
                        .clone(),
                    session_grant_id: session.grant_id().clone(),
                    session_grant_sha256: ArtifactDigest::parse(session.grant_sha256().as_str())
                        .expect("fixture grant digest"),
                    deployment_binding_ref: session.lease().deployment_binding_ref().clone(),
                    deployment_binding_sha256: session.lease().deployment_binding_sha256().clone(),
                    session_floor_epoch: *grant.session_floor_epoch(),
                    grant_issuer_principal_id: grant.issuer_principal_id().clone(),
                    grant_issuer_key_id: grant.issuer_key_id().clone(),
                    grant_issuer_key_epoch: PrincipalKeyEpochV1::new(
                        grant.issuer_key_epoch().get(),
                    )
                    .expect("fixture grant key epoch"),
                    checked_revocation_epoch: current.session_revocations.epoch,
                    checked_revocation_ref: current.session_revocations.revocation_ref.cloned(),
                    checked_revocation_sha256: current
                        .session_revocations
                        .revocation_sha256
                        .cloned(),
                    checked_revocation_checkpoint_ref: capability.checkpoint_ref.clone(),
                    checked_revocation_checkpoint_sha256: ArtifactDigest::parse(
                        capability.checkpoint.canonical_checkpoint_sha256().as_str(),
                    )
                    .expect("fixture capability checkpoint digest"),
                    checked_at: current.now,
                };
                request.issued_at = current.now;
                request.expires_at = UnixTimeV1::new(current.now.get() + 30)
                    .expect("bounded fixture request lifetime");
                request.generation = Some(session.lease().compiled_generation_digest().clone());
                change_request(request);
            };
            with_admitted_start_session_class(
                generation,
                profile,
                suffix,
                seal_current_request,
                admission,
                OperationClassV1::Invoke,
                store,
                RecoveryParents {
                    session,
                    current,
                    lease,
                    signing,
                },
                run,
            )
        },
    )
}

/// Supply the complete unprepared Invoke start to coordinator-level tests.
#[allow(
    dead_code,
    reason = "shared adapter fixtures include this application-only coordinator helper"
)]
pub(crate) fn with_invoke_start_input<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    run: impl FnOnce(
        LocalOperationStartInput<'_>,
        &CommonRequestEnvelopeV1,
        ControlStateCheckpointPreimageV1,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_invoke_start_input_at_store(
        generation,
        profile,
        suffix,
        change_request,
        &store_coordinate(),
        run,
    )
}

/// Supply an unprepared Invoke start whose admitted deployment uses one store.
#[allow(clippy::too_many_arguments)]
pub(crate) fn with_invoke_start_input_at_store<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    store: &AuthorityStoreCoordinate,
    run: impl FnOnce(
        LocalOperationStartInput<'_>,
        &CommonRequestEnvelopeV1,
        ControlStateCheckpointPreimageV1,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_operation_start_input_at_store(
        generation,
        profile,
        suffix,
        change_request,
        OperationClassV1::Invoke,
        store,
        run,
    )
}

/// Supply an unprepared Invoke start against an admitted deployment authority.
#[allow(clippy::too_many_arguments)]
pub(crate) fn with_invoke_start_input_at_store_and_authorities<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    store: &AuthorityStoreCoordinate,
    trust_ref: &ResidentRecordRefV1,
    trust: &PrincipalTrustCheckpointV1,
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    runtime_ref: &ResidentRecordRefV1,
    runtime: &DeploymentRuntimeAuthorityV1,
    run: impl FnOnce(
        LocalOperationStartInput<'_>,
        &CommonRequestEnvelopeV1,
        ControlStateCheckpointPreimageV1,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    let operation = OperationLeaseRequestV1 {
        operation_lease_id: OperationLeaseIdV1::parse("start-lease").expect("lease"),
        operation: operation_id(),
        capabilities: vec![CapabilityIdV1::parse("read").expect("capability")],
        effects: vec![EffectClass::Compute],
        read_set: vec![record_ref("read-a")],
        evidence_obligations: vec![record_ref("evidence-a")],
        budget_units: LeaseBudgetUnitsV1::new(5).expect("budget"),
        timeout_millis: LeaseTimeoutMillisV1::new(500).expect("timeout"),
        expires_at: UnixTimeV1::new(90).expect("expiry"),
        invocation_correlation: InvocationCorrelationV1::Unobserved {
            reason: ResidentReasonV1::parse("not observed by this fixture").expect("reason"),
        },
    };
    with_local_start_session_context_at_store_and_authorities(
        generation,
        profile,
        operation,
        LocalStartContext::Inherited,
        store,
        trust_ref,
        trust,
        binding_ref,
        binding,
        runtime_ref,
        runtime,
        |session, current, lease, signing| {
            let capability = current
                .capability_revocations
                .as_ref()
                .expect("fixture capability frontier");
            with_admitted_start_input_class(
                generation,
                profile,
                suffix,
                |request| {
                    request.expires_at = UnixTimeV1::new(current.now.get() + 30)
                        .expect("bounded fixture request lifetime");
                    request.generation = Some(session.lease().compiled_generation_digest().clone());
                    request.session_admission.checked_revocation_checkpoint_ref =
                        capability.checkpoint_ref.clone();
                    request
                        .session_admission
                        .checked_revocation_checkpoint_sha256 = ArtifactDigest::parse(
                        capability.checkpoint.canonical_checkpoint_sha256().as_str(),
                    )
                    .expect("fixture capability checkpoint digest");
                    change_request(request);
                },
                OperationClassV1::Invoke,
                store,
                RecoveryParents {
                    session,
                    current,
                    lease,
                    signing,
                },
                |input, request, checkpoint, _, _, parents| {
                    run(input, request, checkpoint, parents)
                },
            )
        },
    )
}

/// Supply one complete unprepared operation start for its exact class and store.
#[allow(clippy::too_many_arguments)]
pub(crate) fn with_operation_start_input_at_store<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    operation_class: OperationClassV1,
    store: &AuthorityStoreCoordinate,
    run: impl FnOnce(
        LocalOperationStartInput<'_>,
        &CommonRequestEnvelopeV1,
        ControlStateCheckpointPreimageV1,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_operation_start_input_at_store_and_native(
        generation,
        profile,
        suffix,
        change_request,
        operation_class,
        store,
        None,
        run,
    )
}

/// Supply the same actual start owners with an observed native session and tool.
#[allow(clippy::too_many_arguments)]
pub(crate) fn with_operation_start_input_at_store_and_native<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    operation_class: OperationClassV1,
    store: &AuthorityStoreCoordinate,
    native_tool: Option<&str>,
    run: impl FnOnce(
        LocalOperationStartInput<'_>,
        &CommonRequestEnvelopeV1,
        ControlStateCheckpointPreimageV1,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    let operation = OperationLeaseRequestV1 {
        operation_lease_id: OperationLeaseIdV1::parse("start-lease").expect("lease"),
        operation: operation_id(),
        capabilities: vec![CapabilityIdV1::parse("read").expect("capability")],
        effects: vec![EffectClass::Compute],
        read_set: vec![record_ref("read-a")],
        evidence_obligations: vec![record_ref("evidence-a")],
        budget_units: LeaseBudgetUnitsV1::new(5).expect("budget"),
        timeout_millis: LeaseTimeoutMillisV1::new(500).expect("timeout"),
        expires_at: UnixTimeV1::new(90).expect("expiry"),
        invocation_correlation: match native_tool {
            Some(tool) => InvocationCorrelationV1::Observed {
                invocation_correlation_id: InvocationCorrelationIdV1::parse(
                    "native-start-correlation",
                )
                .unwrap(),
                native_invocation_id: NativeInvocationIdV1::parse(tool).unwrap(),
            },
            None => InvocationCorrelationV1::Unobserved {
                reason: ResidentReasonV1::parse("not observed by this fixture").expect("reason"),
            },
        },
    };
    with_local_start_session_context_at_store(
        generation,
        profile,
        operation,
        if native_tool.is_some() {
            LocalStartContext::ObservedNative
        } else {
            LocalStartContext::Inherited
        },
        LocalStartSessionStore {
            store,
            registered_generation_heads: false,
            existing_authorities: None,
        },
        |session, current, lease, signing| {
            let parents = RecoveryParents {
                session,
                current,
                lease,
                signing,
            };
            with_admitted_start_input_class(
                generation,
                profile,
                suffix,
                |request| {
                    if let Some(tool) = native_tool {
                        let provider = generation
                            .repository(session.lease().repository_id())
                            .unwrap()
                            .render()
                            .support
                            .iter()
                            .find(|row| {
                                row.execution_path_witness.selection
                                    == *session.lease().execution_path()
                            })
                            .unwrap()
                            .execution_path_witness
                            .terminal_provider_binding
                            .clone()
                            .unwrap();
                        let route = super::hooks::RepositoryHookRoutingV1::new(
                            LifecycleEvent::PreToolUse,
                            profile.workspace_id.clone(),
                            session.lease().repository_id().clone(),
                            provider,
                        )
                        .unwrap();
                        let native = serde_json::json!({"hook_event_name":"PreToolUse","session_id":"native-session","tool_use_id":tool,"tool_name":"Read","tool_input":{"file_path":"source.rs"}});
                        request.body = super::resident::ResidentNativeToolStart::prepare(
                            route,
                            native.to_string().as_bytes(),
                        )
                        .unwrap()
                        .body()
                        .clone();
                    }
                    change_request(request);
                },
                operation_class,
                store,
                parents,
                |input, request, checkpoint, _, _, parents| {
                    run(input, request, checkpoint, parents)
                },
            )
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn with_start_context_class<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    context_selection: LocalStartContext,
    operation_class: OperationClassV1,
    registered_generation_heads: bool,
    existing_authorities: Option<LocalStartExistingAuthorities<'_>>,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_start_context_class_at_store(
        generation,
        profile,
        suffix,
        change_request,
        admission,
        context_selection,
        operation_class,
        &store_coordinate(),
        registered_generation_heads,
        existing_authorities,
        run,
    )
}

#[allow(clippy::too_many_arguments)]
fn with_start_context_class_at_store<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    context_selection: LocalStartContext,
    operation_class: OperationClassV1,
    store: &AuthorityStoreCoordinate,
    registered_generation_heads: bool,
    existing_authorities: Option<LocalStartExistingAuthorities<'_>>,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    let mut operation = OperationLeaseRequestV1 {
        operation_lease_id: OperationLeaseIdV1::parse("start-lease").expect("lease"),
        operation: operation_id(),
        capabilities: vec![CapabilityIdV1::parse("read").expect("capability")],
        effects: vec![EffectClass::Compute],
        read_set: vec![record_ref("read-a")],
        evidence_obligations: vec![record_ref("evidence-a")],
        budget_units: LeaseBudgetUnitsV1::new(5).expect("budget"),
        timeout_millis: LeaseTimeoutMillisV1::new(500).expect("timeout"),
        expires_at: UnixTimeV1::new(90).expect("expiry"),
        invocation_correlation: InvocationCorrelationV1::Unobserved {
            reason: ResidentReasonV1::parse("not observed by this fixture").expect("reason"),
        },
    };
    if matches!(context_selection, LocalStartContext::ObservedNative) {
        operation.invocation_correlation = InvocationCorrelationV1::Observed {
            invocation_correlation_id: InvocationCorrelationIdV1::parse("native-hook-correlation")
                .unwrap(),
            native_invocation_id: NativeInvocationIdV1::parse("native-hook-call").unwrap(),
        };
    }
    if !context_selection.has_read_authority() {
        operation.read_set.clear();
    }
    if !context_selection.has_evidence_obligations() {
        operation.evidence_obligations.clear();
    }
    with_local_start_session_context_at_store(
        generation,
        profile,
        operation,
        context_selection,
        LocalStartSessionStore {
            store,
            registered_generation_heads,
            existing_authorities,
        },
        |session, current, lease, signing| {
            with_admitted_start_session_class(
                generation,
                profile,
                suffix,
                change_request,
                admission,
                operation_class,
                store,
                RecoveryParents {
                    session,
                    current,
                    lease,
                    signing,
                },
                run,
            )
        },
    )
}

/// Reuse the exact request and local-start builder with an independently admitted session.
#[allow(
    dead_code,
    reason = "shared adapter fixtures do not exercise admitted session assembly"
)]
pub(crate) fn with_admitted_start_session<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    parents: RecoveryParents<'_>,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_admitted_start_session_class(
        generation,
        profile,
        suffix,
        change_request,
        admission,
        OperationClassV1::Invoke,
        &store_coordinate(),
        parents,
        run,
    )
}

#[allow(clippy::too_many_arguments)]
fn with_admitted_start_session_class<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    admission: StartAdmission<'_>,
    operation_class: OperationClassV1,
    store: &AuthorityStoreCoordinate,
    parents: RecoveryParents<'_>,
    run: impl FnOnce(
        Result<PreparedLocalOperationStart, LocalOperationStartError>,
        &CommonRequestEnvelopeV1,
        usize,
        usize,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_admitted_start_input_class(
        generation,
        profile,
        suffix,
        change_request,
        operation_class,
        store,
        parents,
        |input, request, checkpoint, owner_calls, sign_calls, parents| {
            let publication_scope = input.publication_policy.target_scope;
            let publication_classification = input.publication_policy.classification;
            let publication_visibility = input.publication_policy.visibility;
            let publication_disclosure = input.publication_policy.disclosure_ceiling;
            let publication_payload_policy = input.publication_policy.payload_resolution_policy;
            let publication_actor = input.publication_policy.actor_provenance;
            let admit_owner =
                |actual: &CommonRequestEnvelopeV1,
                 _: &AdmittedAmbientSession,
                 _: &AmbientSessionAdmissionContext<'_>,
                 _: &OperationHandlePreimageV1,
                 _: &OperationLeaseV1,
                 selected: &super::semantic::ResolvedOperation,
                 policy: &LocalOperationPublicationPolicyInput<'_>,
                 _: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>| {
                    owner_calls.set(owner_calls.get() + 1);
                    assert_eq!(actual.body(), request.body());
                    assert_eq!(selected.capabilities().len(), 2);
                    assert_eq!(policy.target_scope, publication_scope);
                    assert_eq!(policy.classification, publication_classification);
                    assert_eq!(policy.visibility, publication_visibility);
                    assert_eq!(policy.disclosure_ceiling, publication_disclosure);
                    assert_eq!(policy.payload_resolution_policy, publication_payload_policy);
                    assert_eq!(policy.actor_provenance, publication_actor);
                    if matches!(
                        admission,
                        StartAdmission::RejectedOwnerModel
                            | StartAdmission::NativeInput {
                                reject_owner: true,
                                ..
                            }
                    ) {
                        Err(InvalidResidentRecordV1::ContextualAdmission {
                            record: "test local owner",
                            invariant: "selected authority unavailable",
                        })
                    } else {
                        Ok(())
                    }
                };
            let sign_handle =
                |_: &[u8], _: &OperationHandlePreimageV1, _: &OperationHandleDigest| {
                    sign_calls.set(sign_calls.get() + 1);
                    Ok(signature())
                };
            let sign_observation =
                |_: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
                    sign_calls.set(sign_calls.get() + 1);
                    Ok(signature())
                };
            let result = match (operation_class, admission) {
                (OperationClassV1::Invoke, StartAdmission::NativeInput { registry, .. }) => {
                    prepare_local_start(
                        input,
                        None,
                        OperationClassV1::Invoke,
                        Some(registry),
                        &mut ProvingAuthority,
                        admit_owner,
                        sign_handle,
                        sign_observation,
                    )
                }
                (OperationClassV1::Invoke, StartAdmission::AcceptedOwnerModel)
                | (OperationClassV1::Invoke, StartAdmission::RejectedOwnerModel) => {
                    prepare_local_start(
                        input,
                        None,
                        OperationClassV1::Invoke,
                        None,
                        &mut ProvingAuthority,
                        admit_owner,
                        sign_handle,
                        sign_observation,
                    )
                }
                (OperationClassV1::Consult, StartAdmission::EvidenceConsult { build_collection, limits }) => {
                    super::resident::local_operation_start_test_support::prepare_evidence_consult_start(
                        input, build_collection, limits, &mut ProvingAuthority, admit_owner, sign_handle, sign_observation,
                    )
                }
                (OperationClassV1::Consult, StartAdmission::AcceptedOwnerModel)
                | (OperationClassV1::Consult, StartAdmission::RejectedOwnerModel) => {
                    prepare_local_start(
                        input,
                        None,
                        OperationClassV1::Consult,
                        None,
                        &mut ProvingAuthority,
                        admit_owner,
                        sign_handle,
                        sign_observation,
                    )
                }
                _ => panic!("unsupported local start fixture class/admission pair"),
            };
            let _ = checkpoint;
            run(
                result,
                request,
                owner_calls.get(),
                sign_calls.get(),
                parents,
            )
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn with_admitted_start_input_class<R>(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    suffix: &str,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    operation_class: OperationClassV1,
    store: &AuthorityStoreCoordinate,
    parents: RecoveryParents<'_>,
    run: impl FnOnce(
        LocalOperationStartInput<'_>,
        &CommonRequestEnvelopeV1,
        ControlStateCheckpointPreimageV1,
        &Cell<usize>,
        &Cell<usize>,
        RecoveryParents<'_>,
    ) -> R,
) -> R {
    let RecoveryParents {
        session,
        current,
        lease,
        signing,
    } = parents;
    let mut request = request_fixture::request_preimage(suffix, &request_fixture::action(suffix));
    request.envelope_schema_sha256 = schema_digest(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID);
    request.workspace_id =
        ResidentWorkspaceIdV1::parse(profile.workspace_id.as_str()).expect("principal workspace");
    request.capability_id = CapabilityIdV1::parse("read").expect("capability");
    request.operation = lease.operation().clone();
    request.caller = RequestCallerV1 {
        identity: current.subject.identity.clone(),
        role: current.subject.role.clone(),
        surface: current.subject.surface.clone(),
        signer_key_id: current.subject.subject_key_id.clone(),
        signer_key_epoch: current.subject.subject_key_epoch,
    };
    request.signing_authority_binding = signing.clone();
    request.route.origin_principal_id = session.lease().principal_id().clone();
    request.route.origin_node_id = current.route_use.origin_node_id.clone();
    request.route.destination_principal_id = current.route_use.destination_principal_id.clone();
    request.route.destination_node_id = current.route_use.destination_node_id.cloned();
    request.route.route_id = current.route_use.route.route_id.clone();
    request.route.route_generation = current.route_use.route.route_generation;
    request.route.route_binding_schema_id = current.route_use.route.route_binding_schema_id.clone();
    request.route.route_binding_ref = current.route_use.route.route_binding_ref.clone();
    request.route.route_binding_sha256 = current.route_use.route.route_binding_sha256.clone();
    request.route.contract_set_ref = current.route_use.contract_set.contract_set_ref.clone();
    request.route.contract_set_sha256 =
        ArtifactDigest::parse(current.route_use.contract_set.contract_set_sha256.as_str())
            .expect("contract digest");
    request.session_admission.session_grant_id = session.grant_id().clone();
    request.session_admission.session_admission_id = session.lease().session_admission_id().clone();
    request.session_admission.session_admission_lease_digest =
        session.lease().session_admission_lease_digest().clone();
    request.session_admission.session_grant_sha256 =
        ArtifactDigest::parse(session.grant_sha256().as_str()).expect("grant digest");
    request.session_admission.deployment_binding_ref =
        session.lease().deployment_binding_ref().clone();
    request.session_admission.deployment_binding_sha256 =
        session.lease().deployment_binding_sha256().clone();
    let grant = current.grant.as_ref().expect("admitted grant").grant;
    request.session_admission.session_floor_epoch = *grant.session_floor_epoch();
    request.session_admission.grant_issuer_principal_id = grant.issuer_principal_id().clone();
    request.session_admission.grant_issuer_key_id = grant.issuer_key_id().clone();
    request.session_admission.grant_issuer_key_epoch =
        PrincipalKeyEpochV1::new(grant.issuer_key_epoch().get()).expect("grant epoch");
    request.session_admission.checked_revocation_epoch = current.session_revocations.epoch;
    request.session_admission.checked_revocation_ref =
        current.session_revocations.revocation_ref.cloned();
    request.session_admission.checked_revocation_sha256 =
        current.session_revocations.revocation_sha256.cloned();
    request.session_admission.checked_at = current.now;
    request.scope.principal_scope = session.lease().principal_id().clone();
    request.scope.controller_scope = session.lease().controller_id().clone();
    request.scope.relationship_scope = session.lease().relationship_scope().clone();
    request.issued_at = current.now;
    request.body.payload = serde_json::json!({"query": "read-a"});
    request.body.schema_id = SchemaId::parse("example/read-query.v1").expect("test body schema");
    request.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&request.body.payload)
            .expect("body")
            .as_bytes(),
    );
    change_request(&mut request);
    let request = request_fixture::request(&request);
    let binding = request
        .local_request_binding()
        .expect("local fixture binding");
    let schema = schema_digest(OPERATION_HANDLE_V1_SCHEMA_ID);
    let lease_ref = operation_lease_ref();
    let handle_context = OperationHandleAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &schema,
            signing_authority: signing,
        },
        request_binding: &binding,
        operation_lease_ref: &lease_ref,
        operation_lease: lease,
        parent_operation: None,
        compiled_generation_sha256: generation.record().compiled_generation_digest(),
        activation_fence_sha256: session.lease().activation_fence_digest(),
        instance_sha256: generation.record().instance_digest(),
    };
    let supported_profile = self::profile();
    let mut checkpoint = checkpoint_preimage_at_store(1, None, store);
    checkpoint.checkpoint_profile_sha256 =
        super::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
    checkpoint.checkpoint_domain_id =
        super::resident::SupportedOperationCheckpointProfile::conflict_domain(&request).unwrap();
    let checkpoint_authority = checkpoint_authority(&checkpoint);
    let owner_calls = Cell::new(0);
    let sign_calls = Cell::new(0);
    let handle = OperationHandlePreimageV1 {
        schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).expect("handle schema"),
        schema_sha256: schema.clone(),
        operation_handle_id: OperationHandleIdV1::parse(format!("operation-{suffix}"))
            .expect("operation"),
        operation_class,
        parent_operation: None,
        request_binding: binding.clone(),
        operation_lease_ref: lease_ref.clone(),
        operation_lease_sha256: lease.operation_lease_digest().clone(),
        compiled_generation_sha256: generation.record().compiled_generation_digest().clone(),
        activation_fence_sha256: session.lease().activation_fence_digest().clone(),
        instance_sha256: generation.record().instance_digest().clone(),
        created_at: current.now,
        expires_at: UnixTimeV1::new(80).expect("expiry"),
        retain_until: UnixTimeV1::new(120).expect("retention"),
        deployment_runtime_authority_ref: session.runtime_authority_ref().clone(),
        deployment_runtime_authority_sha256: ArtifactDigest::parse(
            session.runtime_authority_sha256().as_str(),
        )
        .expect("runtime digest"),
        issuer_principal_id: signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing.signer_key_id()).expect("signer"),
        issuer_key_epoch: *signing.signer_key_epoch(),
    };
    let publication_scope = EventScopeV1 {
        principal_scope: request.scope().principal_scope.clone(),
        controller_scope: request.scope().controller_scope.clone(),
        relationship_scope: request.scope().relationship_scope.clone(),
        environment: DeploymentEnvironmentV1::parse(request.scope().environment.as_str())
            .expect("publication environment"),
    };
    let publication_classification =
        DataClassificationV1::parse(request.scope().data_classification.as_str())
            .expect("publication classification");
    let publication_visibility =
        InteractionVisibilityV1::parse("private").expect("publication visibility");
    let publication_disclosure =
        InteractionDisclosureCeilingV1::parse("private").expect("publication disclosure");
    let publication_payload_policy =
        PayloadResolutionPolicyV1::parse("reference-only").expect("payload resolution policy");
    let publication_actor = publication_actor(suffix);
    let input = LocalOperationStartInput {
        checkpoint_profile: &supported_profile,
        request_ref: record_ref(&format!("request/{suffix}")),
        request: &request,
        handle_ref: record_ref(&format!("handle/{suffix}")),
        handle,
        context: &handle_context,
        session,
        current,
        generation,
        publication_policy: LocalOperationPublicationPolicyInput {
            target_scope: &publication_scope,
            classification: &publication_classification,
            visibility: &publication_visibility,
            disclosure_ceiling: &publication_disclosure,
            payload_resolution_policy: &publication_payload_policy,
            actor_provenance: &publication_actor,
        },
        checkpoint_authority: &checkpoint_authority,
        observation_ref: record_ref(&format!("observation/{suffix}")),
        observation_schema: schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
    };
    run(
        input,
        &request,
        checkpoint.clone(),
        &owner_calls,
        &sign_calls,
        RecoveryParents {
            session,
            current,
            lease,
            signing,
        },
    )
}
