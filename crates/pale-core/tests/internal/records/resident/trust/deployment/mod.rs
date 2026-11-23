//! External law tests for the pure deployment-trust contract graph.

use super::*;
use serde::{de::DeserializeOwned, Serialize};

fn hex(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

macro_rules! digest {
    ($type:ty, $byte:literal) => {
        <$type>::parse(hex($byte)).expect("valid test digest")
    };
}

mod authorities;
mod genesis;

fn schema_id(value: &str) -> SchemaId {
    SchemaId::parse(value).expect("valid schema id")
}

fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(hex(byte)).expect("valid schema digest")
}

fn resident_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("valid resident record reference")
}

fn principal(value: &str) -> ResidentPrincipalIdV1 {
    ResidentPrincipalIdV1::parse(value).expect("valid principal")
}

fn key(value: &str) -> ResidentKeyIdV1 {
    ResidentKeyIdV1::parse(value).expect("valid key")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("valid signature")
}

fn public_key() -> Ed25519PublicKeyV1 {
    Ed25519PublicKeyV1::parse("A".repeat(43)).expect("valid public key")
}

fn wire<T: Serialize, W: DeserializeOwned>(record: &T) -> W {
    serde_json::from_value(serde_json::to_value(record).expect("record serializes"))
        .expect("record shape decodes as its wire")
}

fn denied(record: &'static str, invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission { record, invariant }
}

fn contract_entry(id: &str, reference: &str, digest_byte: char) -> ContractSetEntryV1 {
    ContractSetEntryV1::new(
        schema_id(id),
        resident_ref(reference),
        schema_digest(digest_byte),
    )
}

fn contract_set_preimage(entries: Vec<ContractSetEntryV1>) -> ContractSetPreimageV1 {
    ContractSetPreimageV1 {
        schema_id: schema_id(CONTRACT_SET_V1_SCHEMA),
        schema_sha256: schema_digest('1'),
        contract_set_id: ContractSetIdV1::parse("contract-set-1").expect("contract set id"),
        entries,
        compatibility_profile_ref: resident_ref("compatibility-profile-1"),
        compatibility_profile_sha256: digest!(CompatibilityProfileDigest, 'a'),
    }
}

fn terms_preimage() -> DeploymentTermsPreimageV1 {
    DeploymentTermsPreimageV1 {
        schema_id: schema_id(DEPLOYMENT_TERMS_V1_SCHEMA),
        schema_sha256: schema_digest('2'),
        workspace_id: WorkspaceId::parse("workspace-1").expect("workspace id"),
        workspace_commit: WorkspaceCommitV1::parse("commit-1").expect("workspace commit"),
        deployment_id: DeploymentIdV1::parse("deployment-1").expect("deployment id"),
        distribution_release: DistributionReleaseV1::parse("release-1")
            .expect("distribution release"),
        distribution_release_sha256: digest!(DistributionReleaseDigest, 'b'),
        principal_scope: principal("principal-1"),
        controller_scope: ResidentControllerIdV1::parse("controller-1").expect("controller scope"),
        relationship_scope: Some(
            RelationshipScopeV1::parse("relationship-1").expect("relationship scope"),
        ),
        environment: DeploymentEnvironmentV1::parse("production").expect("deployment environment"),
        environment_sha256: digest!(DeploymentEnvironmentDigest, 'c'),
        processing_role_ref: resident_ref("processing-role-1"),
        processing_role_sha256: digest!(ProcessingRoleDigest, 'd'),
        confidentiality_policy_ref: resident_ref("confidentiality-policy-1"),
        confidentiality_policy_sha256: digest!(ConfidentialityPolicyDigest, 'e'),
        data_classification: DataClassificationV1::parse("confidential")
            .expect("data classification"),
        permitted_purposes: vec![
            PermittedPurposeV1::parse("operations").expect("permitted purpose"),
            PermittedPurposeV1::parse("review").expect("permitted purpose"),
        ],
    }
}

fn terms() -> DeploymentTermsV1 {
    DeploymentTermsV1::seal(terms_preimage()).expect("deployment terms seal")
}

fn proposal_preimage(deployment_terms: DeploymentTermsV1) -> DeploymentBindingProposalPreimageV1 {
    let deployment_terms_sha256 = deployment_terms.canonical_deployment_terms_sha256().clone();
    DeploymentBindingProposalPreimageV1 {
        schema_id: schema_id(DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA),
        schema_sha256: schema_digest('3'),
        deployment_proposal_id: DeploymentProposalIdV1::parse("proposal-1").expect("proposal id"),
        deployment_terms,
        deployment_terms_sha256,
        proposed_at: UnixTimeV1::new(10).expect("proposal time"),
        proposal_expires_at: UnixTimeV1::new(20).expect("proposal expiry"),
        compiler_principal_id: principal("principal-1"),
        compiler_key_id: key("compiler-key-1"),
        compiler_key_epoch: PrincipalKeyEpochV1::new(1).expect("compiler key epoch"),
    }
}

fn proposal() -> DeploymentBindingProposalV1 {
    DeploymentBindingProposalV1::seal(proposal_preimage(terms()), signature())
        .expect("deployment proposal seal")
}

fn acceptance_preimage(
    deployment_proposal_ref: ResidentRecordRefV1,
    proposal: &DeploymentBindingProposalV1,
) -> DeploymentAcceptancePreimageV1 {
    DeploymentAcceptancePreimageV1 {
        schema_id: schema_id(DEPLOYMENT_ACCEPTANCE_V1_SCHEMA),
        schema_sha256: schema_digest('4'),
        deployment_acceptance_id: DeploymentAcceptanceIdV1::parse("acceptance-1")
            .expect("acceptance id"),
        deployment_proposal_ref,
        deployment_proposal_sha256: proposal.canonical_proposal_sha256().clone(),
        deployment_terms_sha256: proposal.deployment_terms_sha256().clone(),
        acceptance_authority_ref: resident_ref("acceptance-authority-1"),
        acceptance_authority_sha256: digest!(ArtifactDigest, '1'),
        acceptance_evidence_ref: resident_ref("acceptance-evidence-1"),
        acceptance_evidence_sha256: digest!(ArtifactDigest, '2'),
        decision: DeploymentAcceptanceDecisionV1::Accepted,
        accepted_at: UnixTimeV1::new(12).expect("acceptance time"),
        binding_valid_until: UnixTimeV1::new(30).expect("binding horizon"),
        permitted_binding_issuer_principal_id: principal("principal-1"),
        issuer_principal_id: principal("owner-principal-1"),
        issuer_key_id: key("owner-key-1"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("owner key epoch"),
    }
}

fn acceptance(
    deployment_proposal_ref: &ResidentRecordRefV1,
    proposal: &DeploymentBindingProposalV1,
) -> DeploymentAcceptanceV1 {
    DeploymentAcceptanceV1::seal(
        acceptance_preimage(deployment_proposal_ref.clone(), proposal),
        signature(),
    )
    .expect("deployment acceptance seal")
}

fn binding_preimage(
    proposal_ref: ResidentRecordRefV1,
    proposal: &DeploymentBindingProposalV1,
    acceptance_ref: ResidentRecordRefV1,
    acceptance: &DeploymentAcceptanceV1,
    deployment_terms: DeploymentTermsV1,
) -> DeploymentBindingPreimageV1 {
    let deployment_terms_sha256 = deployment_terms.canonical_deployment_terms_sha256().clone();
    DeploymentBindingPreimageV1 {
        schema_id: schema_id(DEPLOYMENT_BINDING_V1_SCHEMA),
        schema_sha256: schema_digest('5'),
        deployment_binding_id: DeploymentBindingIdV1::parse("binding-1").expect("binding id"),
        deployment_proposal_ref: proposal_ref,
        deployment_proposal_sha256: proposal.canonical_proposal_sha256().clone(),
        deployment_acceptance_ref: acceptance_ref,
        deployment_acceptance_sha256: acceptance.canonical_acceptance_sha256().clone(),
        deployment_terms,
        deployment_terms_sha256,
        issued_at: UnixTimeV1::new(15).expect("binding issue time"),
        expires_at: UnixTimeV1::new(25).expect("binding expiry"),
        issuer_principal_id: principal("principal-1"),
        issuer_key_id: key("binding-key-1"),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("binding key epoch"),
    }
}

fn binding(
    proposal_ref: &ResidentRecordRefV1,
    proposal: &DeploymentBindingProposalV1,
    acceptance_ref: &ResidentRecordRefV1,
    acceptance: &DeploymentAcceptanceV1,
) -> DeploymentBindingV1 {
    DeploymentBindingV1::seal(
        binding_preimage(
            proposal_ref.clone(),
            proposal,
            acceptance_ref.clone(),
            acceptance,
            proposal.deployment_terms().clone(),
        ),
        signature(),
    )
    .expect("deployment binding seal")
}

fn runtime_preimage(
    binding_ref: ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
) -> DeploymentRuntimeAuthorityPreimageV1 {
    DeploymentRuntimeAuthorityPreimageV1 {
        schema_id: schema_id(DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA),
        schema_sha256: schema_digest('6'),
        runtime_authority_id: DeploymentRuntimeAuthorityIdV1::parse("runtime-authority-1")
            .expect("runtime authority id"),
        principal_id: binding.deployment_terms().principal_scope().clone(),
        deployment_binding_ref: binding_ref,
        deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
        control_store_id: ControlStoreIdV1::parse("deployment-store-1").expect("control store id"),
        control_incarnation_id: ControlIncarnationIdV1::parse("incarnation-1")
            .expect("control incarnation id"),
        subject_key_id: key("runtime-key-1"),
        subject_key_epoch: SigningKeyEpochV1::new(1).expect("runtime key epoch"),
        subject_public_key: public_key(),
        permitted_record_schema_ids: vec![
            schema_id("pale/action-envelope.v1"),
            schema_id("pale/effect-receipt.v1"),
        ],
        authority_epoch: DeploymentRuntimeAuthorityEpochV1::new(1)
            .expect("runtime authority epoch"),
        previous_authority_ref: None,
        previous_authority_sha256: None,
        state: PrincipalAuthorityStateV1::Active,
        issued_at: UnixTimeV1::new(16).expect("runtime authority issue time"),
        expires_at: UnixTimeV1::new(24).expect("runtime authority expiry"),
        issuer_principal_id: principal("principal-1"),
        issuer_key_id: key("binding-key-1"),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("root key epoch"),
    }
}

#[test]
fn contract_set_requires_exact_order_unique_ids_and_owner_resolution() {
    let entries = vec![
        contract_entry("contract/a.v1", "schema-a", 'a'),
        contract_entry("contract/b.v1", "schema-b", 'b'),
    ];
    let record = ContractSetV1::seal(contract_set_preimage(entries.clone()))
        .expect("ordered contract set seals");
    let canonical_preimage = contract_set_preimage(entries.clone());
    let expected_fixture = crate::canon::parse_strict(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/deployment/contract-set-preimage.v1.json"
    )))
    .expect("contract-set fixture is strict JSON");
    let expected_vector = crate::canon::to_canonical_string(&expected_fixture)
        .expect("contract-set fixture canonicalizes");
    assert_eq!(
        serde_jcs::to_string(&canonical_preimage).expect("contract set canonicalizes"),
        expected_vector
    );
    let mut resolved = Vec::new();
    let admitted = ContractSetV1::admit_contextual_wire(
        wire(&record),
        &schema_digest('1'),
        |entry| {
            resolved.push(entry.contract_id().as_str().to_owned());
            Ok(())
        },
        |reference, digest, selected| {
            assert_eq!(reference.as_str(), "compatibility-profile-1");
            assert_eq!(digest, &digest!(CompatibilityProfileDigest, 'a'));
            assert_eq!(selected, entries);
            Ok(())
        },
    )
    .expect("owner-resolved contract set admits");
    assert_eq!(admitted.entries().as_slice(), entries.as_slice());
    assert_eq!(resolved, ["contract/a.v1", "contract/b.v1"]);

    let reversed = vec![
        contract_entry("contract/b.v1", "schema-b", 'b'),
        contract_entry("contract/a.v1", "schema-a", 'a'),
    ];
    assert!(ContractSetV1::seal(contract_set_preimage(reversed)).is_err());

    let duplicate_id = vec![
        contract_entry("contract/a.v1", "schema-a", 'a'),
        contract_entry("contract/a.v1", "schema-b", 'b'),
    ];
    assert!(ContractSetV1::seal(contract_set_preimage(duplicate_id)).is_err());

    let changed_schema = ContractSetV1::seal(contract_set_preimage(vec![
        contract_entry("contract/a.v1", "schema-a", 'c'),
        contract_entry("contract/b.v1", "schema-b", 'b'),
    ]))
    .expect("changed schema bytes remain a structurally valid claim");
    let expected_a = schema_digest('a');
    assert!(ContractSetV1::admit_contextual_wire(
        wire(&changed_schema),
        &schema_digest('1'),
        |entry| {
            if entry.contract_id().as_str() == "contract/a.v1"
                && entry.contract_schema_sha256() != &expected_a
            {
                Err(denied("contract set", "selected schema bytes changed"))
            } else {
                Ok(())
            }
        },
        |_, _, _| Ok(()),
    )
    .is_err());

    let missing_reference = ContractSetV1::seal(contract_set_preimage(vec![contract_entry(
        "contract/a.v1",
        "missing-schema",
        'a',
    )]))
    .expect("missing owner bytes remain a structurally valid claim");
    assert!(ContractSetV1::admit_contextual_wire(
        wire(&missing_reference),
        &schema_digest('1'),
        |entry| {
            assert_eq!(entry.contract_schema_ref().as_str(), "missing-schema");
            Err(denied("contract set", "selected schema reference missing"))
        },
        |_, _, _| Ok(()),
    )
    .is_err());

    let mut unknown_profile_preimage = contract_set_preimage(entries.clone());
    unknown_profile_preimage.compatibility_profile_ref =
        resident_ref("compatibility-profile-unknown");
    unknown_profile_preimage.compatibility_profile_sha256 =
        digest!(CompatibilityProfileDigest, 'f');
    let unknown_profile = ContractSetV1::seal(unknown_profile_preimage)
        .expect("unknown profile remains a structurally valid claim");
    assert!(ContractSetV1::admit_contextual_wire(
        wire(&unknown_profile),
        &schema_digest('1'),
        |_| Ok(()),
        |reference, digest, selected| {
            assert_eq!(reference.as_str(), "compatibility-profile-unknown");
            assert_eq!(digest, &digest!(CompatibilityProfileDigest, 'f'));
            assert_eq!(selected, entries);
            Err(denied("contract set", "unknown compatibility profile"))
        },
    )
    .is_err());
}

#[test]
fn deployment_terms_digest_and_owner_resolution_are_both_required() {
    let preimage = terms_preimage();
    let record = DeploymentTermsV1::seal(preimage.clone()).expect("deployment terms seal");
    assert_eq!(
        record.canonical_deployment_terms_sha256(),
        &DeploymentTermsDigest::hash_canonical(&preimage).expect("terms digest")
    );
    assert!(
        DeploymentTermsV1::admit_contextual_wire(wire(&record), &schema_digest('2'), |_| Err(
            denied("deployment terms", "owner resolver rejected")
        ),)
        .is_err()
    );

    let mut unsorted = preimage.clone();
    unsorted.permitted_purposes.reverse();
    assert!(DeploymentTermsV1::seal(unsorted).is_err());
    let mut duplicated = preimage.clone();
    duplicated
        .permitted_purposes
        .push(duplicated.permitted_purposes[1].clone());
    assert!(DeploymentTermsV1::seal(duplicated).is_err());

    let mut changed = serde_json::to_value(&record).expect("terms serialize");
    changed["workspace_commit"] = serde_json::json!("different-commit");
    let changed_wire = serde_json::from_value(changed).expect("changed terms wire");
    assert!(DeploymentTermsV1::admit_contextual_wire(
        changed_wire,
        &schema_digest('2'),
        |_| Ok(()),
    )
    .is_err());
}

#[test]
fn proposal_binds_nested_terms_compiler_key_and_signature_domain() {
    let deployment_terms = terms();
    let record =
        DeploymentBindingProposalV1::seal(proposal_preimage(deployment_terms.clone()), signature())
            .expect("proposal seal");
    let compiler_principal = principal("principal-1");
    let compiler_key = key("compiler-key-1");
    let signer = PrincipalKeyAdmissionContextV1::new(
        &compiler_principal,
        &compiler_key,
        PrincipalKeyEpochV1::new(1).expect("compiler key epoch"),
    );
    DeploymentBindingProposalV1::admit_contextual_wire(
        wire(&record),
        &schema_digest('3'),
        &schema_digest('2'),
        &signer,
        |_| Ok(()),
        |domain, preimage, digest, _| {
            assert_eq!(domain, DEPLOYMENT_BINDING_PROPOSAL_V1_SIGNATURE_DOMAIN);
            assert_eq!(preimage.deployment_terms, deployment_terms);
            assert_eq!(digest, record.canonical_proposal_sha256());
            Ok(())
        },
    )
    .expect("compiler proposal admits");

    let mut wrong_terms_digest = proposal_preimage(terms());
    wrong_terms_digest.deployment_terms_sha256 = digest!(DeploymentTermsDigest, '0');
    assert!(DeploymentBindingProposalV1::seal(wrong_terms_digest, signature()).is_err());

    let wrong_key = key("wrong-compiler-key");
    let wrong_signer = PrincipalKeyAdmissionContextV1::new(
        &compiler_principal,
        &wrong_key,
        PrincipalKeyEpochV1::new(1).expect("compiler key epoch"),
    );
    assert!(DeploymentBindingProposalV1::admit_contextual_wire(
        wire(&record),
        &schema_digest('3'),
        &schema_digest('2'),
        &wrong_signer,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn acceptance_requires_exact_proposal_owner_evidence_chronology_and_callback() {
    let proposal = proposal();
    let proposal_ref = resident_ref("proposal-ref-1");
    let record = acceptance(&proposal_ref, &proposal);
    let authority_ref = resident_ref("acceptance-authority-1");
    let authority_digest = digest!(ArtifactDigest, '1');
    let evidence_ref = resident_ref("acceptance-evidence-1");
    let evidence_digest = digest!(ArtifactDigest, '2');
    let owner = principal("owner-principal-1");
    let owner_key = key("owner-key-1");
    let acceptance_schema_digest = schema_digest('4');
    let context = DeploymentAcceptanceAdmissionContextV1 {
        expected_schema_sha256: &acceptance_schema_digest,
        deployment_proposal_ref: &proposal_ref,
        deployment_proposal: &proposal,
        acceptance_authority_ref: &authority_ref,
        acceptance_authority_sha256: &authority_digest,
        acceptance_evidence_ref: &evidence_ref,
        acceptance_evidence_sha256: &evidence_digest,
        issuer_principal_id: &owner,
        issuer_key_id: &owner_key,
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("owner key epoch"),
    };
    DeploymentAcceptanceV1::admit_contextual_wire(
        wire(&record),
        &context,
        |domain, preimage, digest, _| {
            assert_eq!(domain, DEPLOYMENT_ACCEPTANCE_V1_SIGNATURE_DOMAIN);
            assert_eq!(
                preimage.deployment_terms_sha256,
                *proposal.deployment_terms_sha256()
            );
            assert_eq!(digest, record.canonical_acceptance_sha256());
            Ok(())
        },
    )
    .expect("exact owner acceptance admits");

    let wrong_proposal_ref = resident_ref("proposal-ref-2");
    let wrong_context = DeploymentAcceptanceAdmissionContextV1 {
        deployment_proposal_ref: &wrong_proposal_ref,
        ..context
    };
    assert!(DeploymentAcceptanceV1::admit_contextual_wire(
        wire(&record),
        &wrong_context,
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut wrong_terms = acceptance_preimage(proposal_ref.clone(), &proposal);
    wrong_terms.deployment_terms_sha256 = digest!(DeploymentTermsDigest, '0');
    let wrong_terms = DeploymentAcceptanceV1::seal(wrong_terms, signature())
        .expect("structurally valid mismatched acceptance");
    assert!(DeploymentAcceptanceV1::admit_contextual_wire(
        wire(&wrong_terms),
        &context,
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut too_early = acceptance_preimage(proposal_ref.clone(), &proposal);
    too_early.accepted_at = UnixTimeV1::new(9).expect("early acceptance time");
    let too_early = DeploymentAcceptanceV1::seal(too_early, signature())
        .expect("structurally valid early acceptance");
    assert!(DeploymentAcceptanceV1::admit_contextual_wire(
        wire(&too_early),
        &context,
        |_, _, _, _| Ok(()),
    )
    .is_err());
    assert!(DeploymentAcceptanceV1::admit_contextual_wire(
        wire(&record),
        &context,
        |_, _, _, _| Err(denied("deployment acceptance", "owner callback rejected")),
    )
    .is_err());
}

#[test]
fn revocation_requires_complete_monotone_lineage_and_same_authority_class() {
    let proposal = proposal();
    let proposal_ref = resident_ref("proposal-ref-1");
    let acceptance = acceptance(&proposal_ref, &proposal);
    let acceptance_ref = resident_ref("acceptance-ref-1");
    let mut genesis_preimage = DeploymentAcceptanceRevocationPreimageV1 {
        schema_id: schema_id(DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA),
        schema_sha256: schema_digest('7'),
        deployment_acceptance_revocation_id: DeploymentAcceptanceRevocationIdV1::parse(
            "acceptance-revocation-1",
        )
        .expect("revocation id"),
        deployment_acceptance_ref: acceptance_ref.clone(),
        deployment_acceptance_sha256: acceptance.canonical_acceptance_sha256().clone(),
        deployment_terms_sha256: acceptance.deployment_terms_sha256().clone(),
        revocation_epoch: DeploymentAcceptanceRevocationEpochV1::new(1).expect("revocation epoch"),
        previous_revocation_ref: None,
        previous_revocation_sha256: None,
        previous_revocation_epoch: None,
        effective_at: UnixTimeV1::new(18).expect("revocation time"),
        reason_code: ResidentReasonV1::parse("owner-withdrew-acceptance")
            .expect("revocation reason"),
        issuer_principal_id: principal("owner-principal-1"),
        issuer_key_id: key("owner-key-1"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("owner key epoch"),
    };
    let mut partial = genesis_preimage.clone();
    partial.previous_revocation_ref = Some(resident_ref("partial-predecessor"));
    assert!(DeploymentAcceptanceRevocationV1::seal(partial, signature()).is_err());

    let genesis = DeploymentAcceptanceRevocationV1::seal(genesis_preimage.clone(), signature())
        .expect("revocation genesis seal");
    let genesis_context = DeploymentAcceptanceRevocationAdmissionContextV1 {
        expected_schema_sha256: &schema_digest('7'),
        deployment_acceptance_ref: &acceptance_ref,
        deployment_acceptance: &acceptance,
        current_head: None,
    };
    DeploymentAcceptanceRevocationV1::admit_contextual_wire(
        wire(&genesis),
        &genesis_context,
        |domain, _, digest, _, accepted| {
            assert_eq!(domain, DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(
                digest,
                genesis.canonical_deployment_acceptance_revocation_sha256()
            );
            assert_eq!(accepted, &acceptance);
            Ok(())
        },
    )
    .expect("revocation genesis admits");
    assert!(DeploymentAcceptanceRevocationV1::admit_contextual_wire(
        wire(&genesis),
        &genesis_context,
        |_, _, _, _, _| Err(denied(
            "deployment acceptance revocation",
            "wrong authority class"
        )),
    )
    .is_err());

    let genesis_ref = resident_ref("revocation-ref-1");
    genesis_preimage.revocation_epoch =
        DeploymentAcceptanceRevocationEpochV1::new(2).expect("successor revocation epoch");
    genesis_preimage.previous_revocation_ref = Some(genesis_ref.clone());
    genesis_preimage.previous_revocation_sha256 = Some(
        genesis
            .canonical_deployment_acceptance_revocation_sha256()
            .clone(),
    );
    genesis_preimage.previous_revocation_epoch = Some(*genesis.revocation_epoch());
    let successor = DeploymentAcceptanceRevocationV1::seal(genesis_preimage.clone(), signature())
        .expect("revocation successor seal");
    let successor_context = DeploymentAcceptanceRevocationAdmissionContextV1 {
        expected_schema_sha256: &schema_digest('7'),
        deployment_acceptance_ref: &acceptance_ref,
        deployment_acceptance: &acceptance,
        current_head: Some(DeploymentAcceptanceRevocationHeadV1 {
            revocation_ref: &genesis_ref,
            revocation: &genesis,
        }),
    };
    DeploymentAcceptanceRevocationV1::admit_contextual_wire(
        wire(&successor),
        &successor_context,
        |_, _, _, _, _| Ok(()),
    )
    .expect("exact revocation successor admits");

    let mut delayed_preimage = genesis_preimage.clone();
    delayed_preimage.effective_at = UnixTimeV1::new(19).expect("delayed effective time");
    let delayed = DeploymentAcceptanceRevocationV1::seal(delayed_preimage, signature())
        .expect("delayed successor is structurally valid");
    assert!(DeploymentAcceptanceRevocationV1::admit_contextual_wire(
        wire(&delayed),
        &successor_context,
        |_, _, _, _, _| Ok(()),
    )
    .is_err());

    genesis_preimage.revocation_epoch =
        DeploymentAcceptanceRevocationEpochV1::new(3).expect("skipped revocation epoch");
    let skipped = DeploymentAcceptanceRevocationV1::seal(genesis_preimage, signature())
        .expect("structurally valid skipped successor");
    assert!(DeploymentAcceptanceRevocationV1::admit_contextual_wire(
        wire(&skipped),
        &successor_context,
        |_, _, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn binding_requires_identical_terms_lineage_horizon_and_permitted_principal() {
    let proposal = proposal();
    let proposal_ref = resident_ref("proposal-ref-1");
    let acceptance = acceptance(&proposal_ref, &proposal);
    let acceptance_ref = resident_ref("acceptance-ref-1");
    let record = binding(&proposal_ref, &proposal, &acceptance_ref, &acceptance);
    let binding_principal = principal("principal-1");
    let binding_key = key("binding-key-1");
    let context = DeploymentBindingAdmissionContextV1 {
        expected_schema_sha256: &schema_digest('5'),
        expected_terms_schema_sha256: &schema_digest('2'),
        deployment_proposal_ref: &proposal_ref,
        deployment_proposal: &proposal,
        deployment_acceptance_ref: &acceptance_ref,
        deployment_acceptance: &acceptance,
        signer: PrincipalKeyAdmissionContextV1::new(
            &binding_principal,
            &binding_key,
            PrincipalKeyEpochV1::new(1).expect("binding key epoch"),
        ),
    };
    DeploymentBindingV1::admit_contextual_wire(
        wire(&record),
        &context,
        |_| Ok(()),
        |domain, _, digest, _| {
            assert_eq!(domain, DEPLOYMENT_BINDING_V1_SIGNATURE_DOMAIN);
            assert_eq!(digest, record.canonical_binding_sha256());
            Ok(())
        },
    )
    .expect("exact final binding admits");

    let mut changed_terms_preimage = terms_preimage();
    changed_terms_preimage.environment =
        DeploymentEnvironmentV1::parse("staging").expect("changed environment");
    let changed_terms =
        DeploymentTermsV1::seal(changed_terms_preimage).expect("changed terms seal");
    let changed_terms_binding = DeploymentBindingV1::seal(
        binding_preimage(
            proposal_ref.clone(),
            &proposal,
            acceptance_ref.clone(),
            &acceptance,
            changed_terms,
        ),
        signature(),
    )
    .expect("structurally valid changed-terms binding");
    assert!(DeploymentBindingV1::admit_contextual_wire(
        wire(&changed_terms_binding),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut wrong_lineage_preimage = binding_preimage(
        resident_ref("wrong-proposal-ref"),
        &proposal,
        acceptance_ref.clone(),
        &acceptance,
        proposal.deployment_terms().clone(),
    );
    let wrong_lineage = DeploymentBindingV1::seal(wrong_lineage_preimage.clone(), signature())
        .expect("structurally valid wrong-lineage binding");
    assert!(DeploymentBindingV1::admit_contextual_wire(
        wire(&wrong_lineage),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    wrong_lineage_preimage.deployment_proposal_ref = proposal_ref.clone();
    wrong_lineage_preimage.expires_at = UnixTimeV1::new(31).expect("late binding expiry");
    let late = DeploymentBindingV1::seal(wrong_lineage_preimage, signature())
        .expect("structurally valid late binding");
    assert!(DeploymentBindingV1::admit_contextual_wire(
        wire(&late),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let wrong_principal = principal("wrong-principal");
    let wrong_signer_context = DeploymentBindingAdmissionContextV1 {
        signer: PrincipalKeyAdmissionContextV1::new(
            &wrong_principal,
            &binding_key,
            PrincipalKeyEpochV1::new(1).expect("binding key epoch"),
        ),
        ..context
    };
    assert!(DeploymentBindingV1::admit_contextual_wire(
        wire(&record),
        &wrong_signer_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn runtime_authority_requires_canonical_scope_and_allows_explicit_successor_set() {
    let proposal = proposal();
    let proposal_ref = resident_ref("proposal-ref-1");
    let acceptance = acceptance(&proposal_ref, &proposal);
    let acceptance_ref = resident_ref("acceptance-ref-1");
    let binding = binding(&proposal_ref, &proposal, &acceptance_ref, &acceptance);
    let binding_ref = resident_ref("binding-ref-1");

    let mut empty = runtime_preimage(binding_ref.clone(), &binding);
    empty.permitted_record_schema_ids.clear();
    assert!(DeploymentRuntimeAuthorityV1::seal(empty, signature()).is_err());
    let mut unsorted = runtime_preimage(binding_ref.clone(), &binding);
    unsorted.permitted_record_schema_ids.reverse();
    assert!(DeploymentRuntimeAuthorityV1::seal(unsorted, signature()).is_err());

    let genesis = DeploymentRuntimeAuthorityV1::seal(
        runtime_preimage(binding_ref.clone(), &binding),
        signature(),
    )
    .expect("runtime authority genesis seal");
    let store = ControlStoreIdV1::parse("deployment-store-1").expect("control store id");
    let incarnation =
        ControlIncarnationIdV1::parse("incarnation-1").expect("control incarnation id");
    let root_principal = principal("principal-1");
    let root_key = key("binding-key-1");
    let context = DeploymentRuntimeAuthorityAdmissionContextV1 {
        expected_schema_sha256: &schema_digest('6'),
        deployment_binding_ref: &binding_ref,
        deployment_binding: &binding,
        control_store_id: &store,
        control_incarnation_id: &incarnation,
        signer: PrincipalKeyAdmissionContextV1::new(
            &root_principal,
            &root_key,
            PrincipalKeyEpochV1::new(1).expect("root key epoch"),
        ),
    };
    DeploymentRuntimeAuthorityV1::admit_initial_wire(
        wire(&genesis),
        &context,
        |_| Ok(()),
        |domain, _, digest, _| {
            assert_eq!(domain, DEPLOYMENT_RUNTIME_AUTHORITY_V1_SIGNATURE_DOMAIN);
            assert_eq!(
                digest,
                genesis.canonical_deployment_runtime_authority_sha256()
            );
            Ok(())
        },
    )
    .expect("runtime authority genesis admits");

    let mut external_schema_preimage = runtime_preimage(binding_ref.clone(), &binding);
    external_schema_preimage.permitted_record_schema_ids = vec![schema_id("scope/context.v1")];
    let external_schema = DeploymentRuntimeAuthorityV1::seal(external_schema_preimage, signature())
        .expect("external schema spelling is structurally valid");
    assert!(DeploymentRuntimeAuthorityV1::admit_initial_wire(
        wire(&external_schema),
        &context,
        |schema_id| {
            assert_eq!(schema_id.as_str(), "scope/context.v1");
            Err(denied(
                "deployment runtime authority",
                "record schema is not Pale-native",
            ))
        },
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let genesis_ref = resident_ref("runtime-authority-ref-1");
    let mut successor_preimage = runtime_preimage(binding_ref.clone(), &binding);
    successor_preimage.authority_epoch =
        DeploymentRuntimeAuthorityEpochV1::new(2).expect("successor authority epoch");
    successor_preimage.previous_authority_ref = Some(genesis_ref.clone());
    successor_preimage.previous_authority_sha256 = Some(
        genesis
            .canonical_deployment_runtime_authority_sha256()
            .clone(),
    );
    successor_preimage.issued_at = UnixTimeV1::new(17).expect("successor issue time");
    successor_preimage.permitted_record_schema_ids = vec![
        schema_id("pale/action-envelope.v1"),
        schema_id("pale/effect-admission.v1"),
        schema_id("pale/effect-receipt.v1"),
    ];
    let successor = DeploymentRuntimeAuthorityV1::seal(successor_preimage.clone(), signature())
        .expect("runtime authority successor seal");
    DeploymentRuntimeAuthorityV1::admit_successor_wire(
        wire(&successor),
        &context,
        &genesis_ref,
        &genesis,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("successor may replace its explicit schema allowlist");

    successor_preimage.control_store_id =
        ControlStoreIdV1::parse("different-store").expect("different store");
    let wrong_scope = DeploymentRuntimeAuthorityV1::seal(successor_preimage, signature())
        .expect("structurally valid changed-scope successor");
    let different_store =
        ControlStoreIdV1::parse("different-store").expect("different control store");
    let wrong_scope_context = DeploymentRuntimeAuthorityAdmissionContextV1 {
        control_store_id: &different_store,
        ..context
    };
    assert!(DeploymentRuntimeAuthorityV1::admit_successor_wire(
        wire(&wrong_scope),
        &wrong_scope_context,
        &genesis_ref,
        &genesis,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn deployment_epochs_share_the_exact_portable_jcs_boundary() {
    let maximum = crate::canon::JSON_SAFE_INTEGER_MAX;
    assert_eq!(
        DeploymentAcceptanceRevocationEpochV1::new(maximum)
            .expect("maximum revocation epoch")
            .get(),
        maximum
    );
    assert!(DeploymentAcceptanceRevocationEpochV1::new(maximum + 1).is_err());
    assert!(
        serde_json::from_value::<DeploymentAcceptanceRevocationEpochV1>(serde_json::json!(
            maximum + 1
        ))
        .is_err()
    );
    assert_eq!(
        DeploymentRuntimeAuthorityEpochV1::new(maximum)
            .expect("maximum runtime-authority epoch")
            .get(),
        maximum
    );
    assert!(DeploymentRuntimeAuthorityEpochV1::new(maximum + 1).is_err());
    assert!(
        serde_json::from_value::<DeploymentRuntimeAuthorityEpochV1>(serde_json::json!(maximum + 1))
            .is_err()
    );
}

#[test]
fn deployment_signature_domains_are_exact_and_non_substitutable() {
    assert_eq!(
        DEPLOYMENT_BINDING_PROPOSAL_V1_SIGNATURE_DOMAIN,
        b"PALE_DEPLOYMENT_BINDING_PROPOSAL_V1\0"
    );
    assert_eq!(
        DEPLOYMENT_ACCEPTANCE_V1_SIGNATURE_DOMAIN,
        b"PALE_DEPLOYMENT_ACCEPTANCE_V1\0"
    );
    assert_eq!(
        DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SIGNATURE_DOMAIN,
        b"PALE_DEPLOYMENT_ACCEPTANCE_REVOCATION_V1\0"
    );
    assert_eq!(
        DEPLOYMENT_BINDING_V1_SIGNATURE_DOMAIN,
        b"PALE_DEPLOYMENT_BINDING_V1\0"
    );
    assert_eq!(
        DEPLOYMENT_RUNTIME_AUTHORITY_V1_SIGNATURE_DOMAIN,
        b"PALE_DEPLOYMENT_RUNTIME_AUTHORITY_V1\0"
    );
    assert_ne!(
        DEPLOYMENT_ACCEPTANCE_V1_SIGNATURE_DOMAIN,
        DEPLOYMENT_BINDING_V1_SIGNATURE_DOMAIN
    );
}
