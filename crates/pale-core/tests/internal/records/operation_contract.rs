//! Exercises checked deserialization across the digest-bound operation contract family.

use super::*;
use crate::records::{
    ArtifactDigest, ExecutionPathSelectionIdV1, ExecutionPathSelectionV1, ExecutionPathWitnessV1,
    ExperienceCompositionPlanDigest, ProviderBindingIdentityV1, ProviderBindingRefV1,
    ResponsibilityPathIdentityV1,
};

fn schema_digest() -> crate::records::SchemaDigest {
    crate::records::SchemaDigest::parse("a".repeat(64)).expect("synthetic digest is valid")
}

fn artifact() -> SealedArtifactRef {
    SealedArtifactRef {
        rel_path: "generated/review.md".into(),
        sha256: crate::records::ArtifactDigest::parse("b".repeat(64))
            .expect("synthetic artifact digest is valid"),
    }
}

fn operation_id() -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: crate::records::PackageIdV1::parse("example.review")
            .expect("synthetic Package id is valid"),
        resource_id: crate::records::PackageResourceIdV1::parse("review")
            .expect("synthetic resource id is valid"),
    }
}

fn repository_id() -> LogicalRepositoryIdV1 {
    LogicalRepositoryIdV1::parse("repository").expect("synthetic repository id is valid")
}

fn provider_binding() -> ProviderBindingRefV1 {
    ProviderBindingRefV1 {
        provider_binding_id: "terminal-provider".into(),
        provider_binding_identity: ProviderBindingIdentityV1 {
            provider_binding_sha256: ArtifactDigest::hash_bytes(b"terminal-provider"),
        },
    }
}

fn execution_path_witness() -> ExecutionPathWitnessV1 {
    ExecutionPathWitnessV1 {
        selection: ExecutionPathSelectionV1 {
            id: ExecutionPathSelectionIdV1::parse("direct-terminal")
                .expect("synthetic execution-path identity is valid"),
            experience_path_id: "direct/terminal".into(),
        },
        responsibility_path_identity: ResponsibilityPathIdentityV1 {
            responsibility_path_sha256: ArtifactDigest::hash_bytes(b"direct/terminal"),
        },
        work_delegation_edge_ids: Vec::new(),
        terminal_provider_binding: Some(provider_binding()),
        composition_plan_digest: ExperienceCompositionPlanDigest::hash_canonical(
            &"composition-plan",
        )
        .expect("synthetic composition-plan digest is canonical"),
    }
}

fn contract_preimage() -> OperationContractPreimageV1 {
    OperationContractPreimageV1 {
        schema: SchemaId::parse(OPERATION_CONTRACT_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: 1,
        id: operation_id(),
        dispatch: OperationDispatchV1::Internal,
        mechanics: OperationContractMechanicsV1::SemanticOnly,
        skill: artifact(),
        invocation_policy: None,
        semantic_dependencies: Vec::new(),
        auxiliary_tools: Vec::new(),
        roles: Vec::new(),
    }
}

#[test]
fn operation_family_valid_values_round_trip_through_checked_deserialization() {
    let contract = OperationContractV1::seal(contract_preimage()).unwrap();
    let decoded: OperationContractV1 =
        serde_json::from_value(serde_json::to_value(&contract).unwrap()).unwrap();
    assert_eq!(decoded, contract);

    let catalog = OperationContractCatalogV1::seal(OperationContractCatalogPreimageV1 {
        schema: SchemaId::parse(OPERATION_CATALOG_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: 1,
        profile_digest: ProfileDigest::parse("c".repeat(64)).unwrap(),
        repository_id: repository_id(),
        provider_binding: provider_binding(),
        entries: Vec::new(),
    })
    .unwrap();
    let decoded: OperationContractCatalogV1 =
        serde_json::from_value(serde_json::to_value(&catalog).unwrap()).unwrap();
    assert_eq!(decoded, catalog);

    let route = OperationRouteBindingV1::seal(OperationRouteBindingPreimageV1 {
        schema: SchemaId::parse(OPERATION_ROUTE_BINDING_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: 1,
        profile_digest: catalog.profile_digest.clone(),
        repository_id: catalog.repository_id.clone(),
        execution_path_witness: execution_path_witness(),
        catalog: artifact(),
        catalog_digest: catalog.catalog_digest,
    })
    .unwrap();
    let decoded: OperationRouteBindingV1 =
        serde_json::from_value(serde_json::to_value(&route).unwrap()).unwrap();
    assert_eq!(decoded, route);
}

#[test]
fn checked_deserialization_rejects_redigested_operation_family_invalidity() {
    let mut contract = contract_preimage();
    contract.mechanics = OperationContractMechanicsV1::CapabilityBearing {
        capabilities: Vec::new(),
    };
    let digest = OperationContractDigest::hash_canonical(&contract).unwrap();
    let mut wire = serde_json::to_value(contract).unwrap();
    wire.as_object_mut().unwrap().insert(
        "contract_digest".into(),
        serde_json::to_value(digest).unwrap(),
    );
    assert!(serde_json::from_value::<OperationContractV1>(wire).is_err());

    let entry = OperationCatalogEntryV1 {
        id: operation_id(),
        dispatch: OperationDispatchV1::Internal,
        state: CatalogOperationStateV1::SemanticOnly,
        contract: artifact(),
        contract_digest: OperationContractDigest::parse("e".repeat(64)).unwrap(),
    };
    let catalog = OperationContractCatalogPreimageV1 {
        schema: SchemaId::parse(OPERATION_CATALOG_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: 1,
        profile_digest: ProfileDigest::parse("c".repeat(64)).unwrap(),
        repository_id: repository_id(),
        provider_binding: provider_binding(),
        entries: vec![entry.clone(), entry],
    };
    let digest = OperationCatalogDigest::hash_canonical(&catalog).unwrap();
    let mut wire = serde_json::to_value(catalog).unwrap();
    wire.as_object_mut().unwrap().insert(
        "catalog_digest".into(),
        serde_json::to_value(digest).unwrap(),
    );
    assert!(serde_json::from_value::<OperationContractCatalogV1>(wire).is_err());

    let route = OperationRouteBindingPreimageV1 {
        schema: SchemaId::parse("pale/not-a-route.v1").unwrap(),
        schema_sha256: schema_digest(),
        schema_version: 1,
        profile_digest: ProfileDigest::parse("c".repeat(64)).unwrap(),
        repository_id: repository_id(),
        execution_path_witness: execution_path_witness(),
        catalog: artifact(),
        catalog_digest: OperationCatalogDigest::parse("f".repeat(64)).unwrap(),
    };
    let digest = OperationBindingDigest::hash_canonical(&route).unwrap();
    let mut wire = serde_json::to_value(route).unwrap();
    wire.as_object_mut().unwrap().insert(
        "binding_digest".into(),
        serde_json::to_value(digest).unwrap(),
    );
    assert!(serde_json::from_value::<OperationRouteBindingV1>(wire).is_err());
}

#[test]
fn operation_contract_seal_and_verify_share_initial_v1_validation() {
    let preimage = contract_preimage();
    let mut sealed = OperationContractV1::seal(preimage).expect("valid contract seals");
    sealed.verify().expect("sealed contract verifies");
    sealed.schema_version = 2;

    assert!(matches!(
        sealed.verify(),
        Err(InvalidOperationContractFamilyV1::SchemaCoordinate { .. })
    ));
}

#[test]
fn capability_bearing_contract_cannot_hide_an_empty_requirement_set() {
    let preimage = OperationContractPreimageV1 {
        schema: SchemaId::parse(OPERATION_CONTRACT_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: 1,
        id: operation_id(),
        dispatch: OperationDispatchV1::Internal,
        mechanics: OperationContractMechanicsV1::CapabilityBearing {
            capabilities: Vec::new(),
        },
        skill: artifact(),
        invocation_policy: None,
        semantic_dependencies: Vec::new(),
        auxiliary_tools: Vec::new(),
        roles: Vec::new(),
    };

    assert!(matches!(
        OperationContractV1::seal(preimage),
        Err(InvalidOperationContractFamilyV1::EmptyCapabilities)
    ));
}

#[test]
fn catalog_and_route_verify_their_own_schema_and_digest() {
    let catalog = OperationContractCatalogV1::seal(OperationContractCatalogPreimageV1 {
        schema: SchemaId::parse(OPERATION_CATALOG_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: 1,
        profile_digest: ProfileDigest::parse("c".repeat(64)).unwrap(),
        repository_id: repository_id(),
        provider_binding: provider_binding(),
        entries: Vec::new(),
    })
    .expect("valid catalog seals");
    catalog.verify().expect("sealed catalog verifies");

    let mut route = OperationRouteBindingV1::seal(OperationRouteBindingPreimageV1 {
        schema: SchemaId::parse(OPERATION_ROUTE_BINDING_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: 1,
        profile_digest: catalog.profile_digest.clone(),
        repository_id: catalog.repository_id.clone(),
        execution_path_witness: execution_path_witness(),
        catalog: artifact(),
        catalog_digest: catalog.catalog_digest,
    })
    .expect("valid route binding seals");
    route.verify().expect("sealed route binding verifies");
    route.binding_digest = OperationBindingDigest::parse("d".repeat(64)).unwrap();

    assert!(matches!(
        route.verify(),
        Err(InvalidOperationContractFamilyV1::DigestMismatch { .. })
    ));
}
