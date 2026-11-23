//! Exercises resource-continuity admission and its non-substitution laws.

use super::*;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("resident record ref")
}

fn coordinate(value: &str, digest_byte: char) -> ContinuityRecordCoordinateV1 {
    ContinuityRecordCoordinateV1::new(
        record_ref(value),
        ArtifactDigest::parse(sha(digest_byte)).expect("artifact digest"),
    )
}

fn authority(owner: &str, value: &str, digest_byte: char) -> ContinuityAuthorityBindingV1 {
    ContinuityAuthorityBindingV1::new(
        OwnerSystemIdV1::parse(owner).expect("owner system"),
        AuthorityEvidenceV1::new(
            record_ref(value),
            ArtifactDigest::parse(sha(digest_byte)).expect("authority digest"),
        ),
    )
}

fn principal() -> ResidentPrincipalIdV1 {
    ResidentPrincipalIdV1::parse("principal-1").expect("principal")
}

fn controller() -> ResidentControllerIdV1 {
    ResidentControllerIdV1::parse("controller-1").expect("controller")
}

fn principal_subject() -> ResourceContinuitySubjectV1 {
    ResourceContinuitySubjectV1::Principal(principal())
}

fn grant(
    subject: ResourceContinuitySubjectV1,
    role: ResourceContinuityAccessRoleV1,
    suffix: &str,
) -> ResourceContinuityGrantV1 {
    ResourceContinuityGrantV1::new(
        subject,
        role,
        coordinate(&format!("grant-{suffix}"), '1'),
        ContinuityOptionalContractV1::Absent {},
        ContinuityOptionalContractV1::Absent {},
    )
}

fn access() -> ResourceContinuityAccessV1 {
    ResourceContinuityAccessV1::new(
        ResourceContinuityPrincipalSetV1::new(vec![principal()]).expect("principal set"),
        ResourceContinuityControllerSetV1::new(vec![controller()]).expect("controller set"),
        ResourceContinuityGrantSetV1::new(vec![
            grant(
                principal_subject(),
                ResourceContinuityAccessRoleV1::Reader,
                "read",
            ),
            grant(
                principal_subject(),
                ResourceContinuityAccessRoleV1::EffectWriter,
                "effect",
            ),
        ])
        .expect("grant set"),
    )
    .expect("access")
}

fn mechanisms() -> ResourceContinuityMechanismTableV1 {
    ResourceContinuityMechanismTableV1::new(
        ResourceContinuityMechanismKindV1::ALL
            .into_iter()
            .map(ResourceContinuityMechanismPolicyV1::forbidden)
            .collect(),
    )
    .expect("complete mechanism table")
}

fn governance() -> ResourceContinuityGovernanceV1 {
    ResourceContinuityGovernanceV1::new(
        coordinate("privacy", '2'),
        coordinate("residency", '3'),
        coordinate("retention", '4'),
        coordinate("revocation", '5'),
        coordinate("deletion", '6'),
        coordinate("legal-hold", '7'),
    )
}

fn preimage() -> ResourceContinuityProfilePreimageV1 {
    ResourceContinuityProfilePreimageV1 {
        schema: SchemaId::parse(RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA).expect("continuity schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("continuity schema digest"),
        schema_version: RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA_VERSION,
        profile_id: ResourceContinuityProfileIdV1::parse("continuity-1").expect("profile id"),
        resource_id: WorkspaceResourceId::parse("resource-1").expect("resource id"),
        authority: ResourceContinuityAuthorityV1::new(
            OwnerSystemIdV1::parse("native-owner").expect("native owner"),
            coordinate("authority-model", '8'),
            authority("source-owner", "source-authority", '9'),
            ContinuityControllerBindingV1::new(
                controller(),
                coordinate("controller-authority", 'b'),
            ),
            authority("effect-owner", "effect-authority", 'c'),
        ),
        access: access(),
        copies: ResourceContinuityCopySetV1::new(Vec::new()).expect("empty copy set"),
        convergence: ResourceConvergenceV1::ExclusiveWriter {
            writer: ResourceContinuityWriterV1::new(
                principal_subject(),
                coordinate("writer-authority", 'd'),
            ),
        },
        mechanisms: mechanisms(),
        recovery: ResourceContinuityRecoveryV1::new(
            RecoveryTimeObjectiveSecondsV1::new(60).expect("RTO"),
            RecoveryPointObjectiveSecondsV1::new(30).expect("RPO"),
            coordinate("recovery-verification", 'e'),
        ),
        governance: governance(),
        scope: ResourceContinuityScopeV1::new(
            ResourceContinuityDeviceScopeSetV1::new(Vec::new()).expect("device scopes"),
            ResourceContinuityMachineScopeSetV1::new(Vec::new()).expect("machine scopes"),
            coordinate("cross-machine-bootstrap", 'f'),
            coordinate("bootstrap-verification", '1'),
        ),
        session_relation: ResourceSessionContinuityV1::Unrelated {},
        cutover: ResourceContinuityCutoverV1::new(
            coordinate("native-cutover-state", '2'),
            coordinate("rollback-contract", '3'),
        ),
        evidence: ResourceContinuityEvidenceV1::new(
            authority("evidence-owner", "evidence-authority", '4'),
            authority("read-back-owner", "read-back-authority", '5'),
            coordinate("evidence-destination", '6'),
            coordinate("read-back-contract", '7'),
        ),
    }
}

fn admitted() -> ResourceContinuityProfileV1 {
    let preimage = preimage();
    let schema = preimage.schema_sha256.clone();
    ResourceContinuityProfileV1::admit(preimage, &schema).expect("valid continuity profile")
}

#[test]
fn canonical_profile_reproduces_its_schema_and_content_identity() {
    let value = admitted();
    value.verify().expect("profile verifies");
    assert_eq!(
        value.schema().as_str(),
        RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA
    );
    assert_eq!(value.resource_id().as_str(), "resource-1");

    let wire = ResourceContinuityProfileWireV1::from(&value);
    let round_trip = ResourceContinuityProfileV1::admit_wire(wire, value.schema_sha256())
        .expect("wire readmission");
    assert_eq!(round_trip, value);
}

#[test]
fn grants_require_authorized_subjects_and_independent_effect_authority() {
    let outsider = ResourceContinuitySubjectV1::Principal(
        ResidentPrincipalIdV1::parse("principal-outsider").expect("outsider"),
    );
    let unauthorized = ResourceContinuityAccessV1::new(
        ResourceContinuityPrincipalSetV1::new(vec![principal()]).expect("principal set"),
        ResourceContinuityControllerSetV1::new(Vec::new()).expect("controller set"),
        ResourceContinuityGrantSetV1::new(vec![
            grant(
                outsider.clone(),
                ResourceContinuityAccessRoleV1::Reader,
                "outsider-read",
            ),
            grant(
                outsider,
                ResourceContinuityAccessRoleV1::EffectWriter,
                "outsider-effect",
            ),
        ])
        .expect("ordered grants"),
    );
    assert!(unauthorized.is_err());

    let reader_only = ResourceContinuityAccessV1::new(
        ResourceContinuityPrincipalSetV1::new(vec![principal()]).expect("principal set"),
        ResourceContinuityControllerSetV1::new(Vec::new()).expect("controller set"),
        ResourceContinuityGrantSetV1::new(vec![grant(
            principal_subject(),
            ResourceContinuityAccessRoleV1::Reader,
            "reader-only",
        )])
        .expect("reader grant"),
    );
    assert!(reader_only.is_err(), "reader must not imply effect writer");
}

#[test]
fn duplicate_or_unordered_grant_keys_are_rejected() {
    let duplicate = ResourceContinuityGrantSetV1::new(vec![
        grant(
            principal_subject(),
            ResourceContinuityAccessRoleV1::Reader,
            "read-a",
        ),
        grant(
            principal_subject(),
            ResourceContinuityAccessRoleV1::Reader,
            "read-b",
        ),
    ]);
    assert!(duplicate.is_err());

    let unordered = ResourceContinuityGrantSetV1::new(vec![
        grant(
            principal_subject(),
            ResourceContinuityAccessRoleV1::EffectWriter,
            "effect",
        ),
        grant(
            principal_subject(),
            ResourceContinuityAccessRoleV1::Reader,
            "read",
        ),
    ]);
    assert!(unordered.is_err());
}

#[test]
fn mechanism_policy_requires_exact_closed_inventory_coverage() {
    let mut missing: Vec<_> = ResourceContinuityMechanismKindV1::ALL
        .into_iter()
        .map(ResourceContinuityMechanismPolicyV1::forbidden)
        .collect();
    missing.pop();
    assert!(ResourceContinuityMechanismTableV1::new(missing).is_err());

    let mut duplicate: Vec<_> = ResourceContinuityMechanismKindV1::ALL
        .into_iter()
        .map(ResourceContinuityMechanismPolicyV1::forbidden)
        .collect();
    duplicate[9] = ResourceContinuityMechanismPolicyV1::forbidden(
        ResourceContinuityMechanismKindV1::MigrationCutover,
    );
    assert!(ResourceContinuityMechanismTableV1::new(duplicate).is_err());
}

#[test]
fn replica_classes_require_matching_logical_holder_scopes() {
    let invalid = ResourceContinuityCopyV1::new(
        ResourceContinuityCopyIdV1::parse("copy-1").expect("copy id"),
        ResourceContinuityCopyClassV1::LocalReplica,
        ContinuityHolderV1::Service(record_ref("remote-service")),
        authority("copy-owner", "copy-authority", '8'),
        ResourceContinuityCopyPolicyV1::new(
            coordinate("copy-scope", '9'),
            coordinate("copy-frontier", 'a'),
            coordinate("copy-retention", 'b'),
            coordinate("copy-revocation", 'c'),
        ),
    );
    assert!(ResourceContinuityCopySetV1::new(vec![invalid]).is_err());
}

#[test]
fn exclusive_writer_must_have_its_own_effect_writer_grant() {
    let mut value = preimage();
    value.convergence = ResourceConvergenceV1::ExclusiveWriter {
        writer: ResourceContinuityWriterV1::new(
            ResourceContinuitySubjectV1::Controller(controller()),
            coordinate("controller-writer", 'd'),
        ),
    };
    let schema = value.schema_sha256.clone();
    assert!(ResourceContinuityProfileV1::admit(value, &schema).is_err());
}

#[test]
fn merge_protocol_requires_a_nonempty_canonical_metadata_set() {
    assert!(ResourceConvergenceMetadataSetV1::new(Vec::new()).is_err());
    assert!(ResourceConvergenceMetadataSetV1::new(vec![
        ResourceConvergenceMetadataV1::CausalClock,
        ResourceConvergenceMetadataV1::CausalClock,
    ])
    .is_err());

    let mut value = preimage();
    value.convergence = ResourceConvergenceV1::MergeProtocol {
        protocol: coordinate("merge-protocol", 'd'),
        merge_owner: authority("merge-owner", "merge-authority", 'e'),
        conflict_owner: authority("conflict-owner", "conflict-authority", 'f'),
        durable_metadata: ResourceConvergenceMetadataSetV1::new(vec![
            ResourceConvergenceMetadataV1::CausalClock,
            ResourceConvergenceMetadataV1::VersionVector,
        ])
        .expect("canonical metadata"),
    };
    let schema = value.schema_sha256.clone();
    ResourceContinuityProfileV1::admit(value, &schema).expect("merge profile");
}

#[test]
fn untrusted_wire_rejects_null_and_unknown_members() {
    let value = admitted();
    let json = serde_json::to_value(&value).expect("profile serializes");

    let mut null_relation = json.clone();
    null_relation["session_relation"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<ResourceContinuityProfileWireV1>(null_relation).is_err());

    let mut unknown = json;
    unknown["owner_resolved"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<ResourceContinuityProfileWireV1>(unknown).is_err());

    let mut nested_unknown = serde_json::to_value(&value).expect("profile serializes");
    nested_unknown["session_relation"]["implicit"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<ResourceContinuityProfileWireV1>(nested_unknown).is_err());

    let mut half_coordinate = serde_json::to_value(&value).expect("profile serializes");
    half_coordinate["authority"]["authority_model"]
        .as_object_mut()
        .expect("authority-model object")
        .remove("record_sha256");
    assert!(serde_json::from_value::<ResourceContinuityProfileWireV1>(half_coordinate).is_err());
}

#[test]
fn same_profile_identity_with_different_policy_bytes_has_a_different_digest() {
    let first = admitted();
    let mut changed = preimage();
    changed.session_relation = ResourceSessionContinuityV1::Bound {
        relation_contract: coordinate("session-relation", 'e'),
    };
    let schema = changed.schema_sha256.clone();
    let second = ResourceContinuityProfileV1::admit(changed, &schema).expect("changed profile");

    assert_eq!(first.profile_id(), second.profile_id());
    assert_ne!(
        first.resource_continuity_profile_digest(),
        second.resource_continuity_profile_digest()
    );
}
