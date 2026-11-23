//! Shared, source-shaped provider-binding fixtures.

use pale_core::records::{
    AdapterPluginRefV1, ArtifactDigest, CapabilityEvidenceCoordinateV1, ContractSchemaIdentityV1,
    LogicalProviderInstanceRefV1, NativeAuthorityOriginRefV1, NativeDataGenerationIdentityV1,
    PaleAdmissionGenerationIdentityV1, ParticipantRefV1, PhysicalDeploymentRefV1,
    ProviderArtifactCoordinatesV1, ProviderAuthorityCoordinatesV1, ProviderBehaviorCoordinatesV1,
    ProviderBindingPreimageV1, ProviderBindingV1, ProviderConformanceCoordinatesV1,
    ProviderExchangeCoordinatesV1, ProviderFamilyRefV1, ProviderGenerationPreimageV1,
    ProviderLifecycleCoordinatesV1, ProviderMappingCoordinatesV1, ProviderProtocolCoordinatesV1,
    ResponsibilityV1, SchemaDigest, SchemaId, RESPONSIBILITY_CONTRACT_V1_SCHEMA_ID,
};

pub(crate) fn digest(label: &str) -> ArtifactDigest {
    ArtifactDigest::hash_bytes(label.as_bytes())
}

pub(crate) fn contract_schema_identity(id: &str) -> ContractSchemaIdentityV1 {
    let descriptor = pale_contracts::schema_descriptor(id).expect("test schema is admitted");
    ContractSchemaIdentityV1 {
        schema_id: SchemaId::parse(descriptor.schema_id).expect("registry id is typed"),
        schema_sha256: SchemaDigest::parse(descriptor.schema_sha256)
            .expect("schema digest is typed"),
    }
}

pub(crate) fn provider_binding(participant: ParticipantRefV1) -> ProviderBindingV1 {
    provider_binding_with_coordinates(participant, "provider-binding-a", "executable-a")
}

pub(crate) fn provider_binding_with_coordinates(
    participant: ParticipantRefV1,
    id: &str,
    executable: &str,
) -> ProviderBindingV1 {
    ProviderBindingV1::seal(ProviderBindingPreimageV1 {
        id: id.into(),
        participant,
        responsibility: ResponsibilityV1::OrganizationalWork,
        responsibility_contract_ids: vec!["responsibility-entry".into()],
        responsibility_schema_identities: vec![contract_schema_identity(
            RESPONSIBILITY_CONTRACT_V1_SCHEMA_ID,
        )],
        communication_actor_handle: None,
        provider_family: ProviderFamilyRefV1 {
            provider_family_id: "provider-family-a".into(),
        },
        logical_provider_instance: LogicalProviderInstanceRefV1 {
            logical_provider_instance_id: "logical-provider-a".into(),
        },
        native_authority_origin: NativeAuthorityOriginRefV1 {
            native_authority_origin_id: "native-authority-a".into(),
        },
        physical_deployment: PhysicalDeploymentRefV1 {
            physical_deployment_id: "deployment-a".into(),
        },
        adapter_plugin: AdapterPluginRefV1 {
            adapter_plugin_id: "adapter-a".into(),
        },
        artifact_coordinates: ProviderArtifactCoordinatesV1 {
            source: "source-a".into(),
            build: "build-a".into(),
            package: "package-a".into(),
            image: "image-a".into(),
            executable: executable.into(),
        },
        protocol_coordinates: ProviderProtocolCoordinatesV1 {
            protocol: "protocol-a".into(),
            configuration: "configuration-a".into(),
            extensions: Vec::new(),
        },
        mapping_coordinates: ProviderMappingCoordinatesV1 {
            terminology: "terminology-a".into(),
            entity: "entity-a".into(),
            field: "field-map-a".into(),
            event: "event-map-a".into(),
        },
        authority_coordinates: ProviderAuthorityCoordinatesV1 {
            field: "field-authority-a".into(),
            native: "native-authority-contract-a".into(),
        },
        lifecycle_coordinates: ProviderLifecycleCoordinatesV1 {
            lifecycle: "lifecycle-a".into(),
            failure: "failure-a".into(),
        },
        exchange_coordinates: ProviderExchangeCoordinatesV1 {
            outbox: "outbox-a".into(),
            cursor: "cursor-a".into(),
            request: "request-a".into(),
            result: "result-a".into(),
            cancel: "cancel-a".into(),
            drain: "drain-a".into(),
        },
        behavior_coordinates: ProviderBehaviorCoordinatesV1 {
            degraded: "degraded-a".into(),
            removal: "removal-a".into(),
        },
        conformance_coordinates: ProviderConformanceCoordinatesV1 {
            conformance: "conformance-a".into(),
            evidence: "evidence-a".into(),
            per_capability_evidence: vec![CapabilityEvidenceCoordinateV1 {
                capability_id: "neutral-capability".into(),
                evidence_contract_id: "capability-evidence-a".into(),
            }],
        },
        generation_coordinates: ProviderGenerationPreimageV1 {
            native_data_generation_identity: NativeDataGenerationIdentityV1 {
                native_data_generation_sha256: digest("native-data-generation-a"),
            },
            pale_admission_generation_identity: PaleAdmissionGenerationIdentityV1 {
                pale_admission_generation_sha256: digest("pale-admission-generation-a"),
            },
        },
    })
    .expect("shared provider binding seals")
}
