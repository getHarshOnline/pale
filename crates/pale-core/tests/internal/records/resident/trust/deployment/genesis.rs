//! Genesis receipt joins preserve the distinction between runtime and checkpoint signers.

use super::*;

struct Fixture {
    binding: DeploymentBindingV1,
    runtime: DeploymentRuntimeAuthorityV1,
    checkpoint_authority: CheckpointSigningAuthorityV1,
    checkpoints: Vec<ControlStateCheckpointV1>,
    checkpoint_values: Vec<ControlStateCheckpointPreimageV1>,
    runtime_value: DeploymentRuntimeAuthorityPreimageV1,
    references: Vec<ResidentRecordRefV1>,
    domains: Vec<ControlCheckpointDomainIdV1>,
    signing: SigningAuthorityBindingV1,
    value: DeploymentControlStoreGenesisReceiptPreimageV1,
}

impl Fixture {
    fn new() -> Self {
        let proposal = proposal();
        let proposal_ref = resident_ref("proposal-1");
        let acceptance = acceptance(&proposal_ref, &proposal);
        let binding = binding(
            &proposal_ref,
            &proposal,
            &resident_ref("acceptance-1"),
            &acceptance,
        );
        let binding_ref = resident_ref("binding-1");
        let mut runtime_value = runtime_preimage(binding_ref.clone(), &binding);
        runtime_value.permitted_record_schema_ids = vec![schema_id(
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
        )];
        let runtime = DeploymentRuntimeAuthorityV1::seal(runtime_value.clone(), signature())
            .expect("runtime fixture");
        let checkpoint_authority = CheckpointSigningAuthorityV1::seal(
            CheckpointSigningAuthorityPreimageV1 {
                schema_id: schema_id(CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA),
                schema_sha256: schema_digest('5'),
                checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1::parse(
                    "checkpoint-authority-1",
                )
                .unwrap(),
                principal_id: principal("principal-1"),
                control_store_id: runtime.control_store_id().clone(),
                control_incarnation_id: runtime.control_incarnation_id().clone(),
                scope: CheckpointSigningScopeV1::Deployment {
                    deployment_binding_ref: binding_ref.clone(),
                    deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
                },
                checkpoint_profile_ref: resident_ref("profile-1"),
                checkpoint_profile_sha256: digest!(CheckpointProfileDigest, '6'),
                allowed_domain_prefixes: vec![
                    CheckpointDomainPrefixV1::parse("effects").unwrap(),
                    CheckpointDomainPrefixV1::parse("operations").unwrap(),
                ],
                subject_key_id: key("checkpoint-key-1"),
                subject_key_epoch: SigningKeyEpochV1::new(1).unwrap(),
                subject_public_key: Ed25519PublicKeyV1::parse("Q".repeat(43)).unwrap(),
                signer_epoch: CheckpointSignerEpochV1::new(1).unwrap(),
                state: PrincipalAuthorityStateV1::Active,
                issued_at: UnixTimeV1::new(16).unwrap(),
                expires_at: UnixTimeV1::new(24).unwrap(),
                previous_authority_ref: None,
                previous_authority_sha256: None,
                issuer_principal_id: principal("principal-1"),
                issuer_key_id: key("binding-key-1"),
                issuer_key_epoch: PrincipalKeyEpochV1::new(1).unwrap(),
            },
            signature(),
        )
        .expect("checkpoint authority fixture");
        let authority_ref = resident_ref("checkpoint-authority-1");
        let domains: Vec<_> = ["effects", "operations"]
            .into_iter()
            .map(|id| ControlCheckpointDomainIdV1::parse(id).unwrap())
            .collect();
        let references: Vec<_> = ["checkpoint-effects", "checkpoint-operations"]
            .into_iter()
            .map(resident_ref)
            .collect();
        let checkpoint_values: Vec<_> = domains
            .iter()
            .enumerate()
            .map(|(index, domain)| ControlStateCheckpointPreimageV1 {
                schema_id: schema_id(CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                schema_sha256: schema_digest('7'),
                control_checkpoint_id: ControlCheckpointIdV1::parse(format!("checkpoint-{index}"))
                    .unwrap(),
                control_store_id: runtime.control_store_id().clone(),
                control_incarnation_id: runtime.control_incarnation_id().clone(),
                checkpoint_domain_id: domain.clone(),
                domain_epoch: ControlDomainEpochV1::new(1).unwrap(),
                domain_sequence: ControlDomainSequenceV1::new(1).unwrap(),
                previous_control_checkpoint_ref: None,
                previous_control_checkpoint_sha256: None,
                checkpoint_profile_ref: checkpoint_authority.checkpoint_profile_ref().clone(),
                checkpoint_profile_sha256: checkpoint_authority.checkpoint_profile_sha256().clone(),
                control_schema_version: 1,
                committed_transaction_sequence: ControlTransactionSequenceV1::new(9).unwrap(),
                state_root_sha256: digest!(ArtifactDigest, 'a'),
                leaf_count: ControlLeafCountV1::new(1).unwrap(),
                changed_leaf_keys: vec![ChangedControlLeafKeyV1 {
                    record_class: ControlRecordClassV1::parse("initial-state").unwrap(),
                    scope_key_sha256: digest!(ArtifactDigest, 'b'),
                    logical_key: ControlLogicalKeyV1::parse("initial").unwrap(),
                }],
                unresolved_custody_root_sha256: digest!(ArtifactDigest, 'c'),
                issued_at: UnixTimeV1::new(18).unwrap(),
                checkpoint_signing_authority_ref: authority_ref.clone(),
                checkpoint_signing_authority_sha256: checkpoint_authority
                    .canonical_checkpoint_signing_authority_sha256()
                    .clone(),
                issuer_principal_id: checkpoint_authority.principal_id().clone(),
                issuer_key_id: checkpoint_authority.subject_key_id().clone(),
                issuer_key_epoch: *checkpoint_authority.subject_key_epoch(),
            })
            .collect();
        let checkpoints: Vec<_> = checkpoint_values
            .iter()
            .cloned()
            .map(|value| {
                ControlStateCheckpointV1::seal(value, signature()).expect("checkpoint fixture")
            })
            .collect();
        let value = DeploymentControlStoreGenesisReceiptPreimageV1 {
            schema_id: schema_id(DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA),
            schema_sha256: schema_digest('8'),
            genesis_id: DeploymentControlStoreGenesisIdV1::parse("genesis-1").unwrap(),
            principal_id: principal("principal-1"),
            workspace_id: binding.deployment_terms().workspace_id().clone(),
            deployment_binding_ref: binding_ref.clone(),
            deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
            control_store_id: runtime.control_store_id().clone(),
            control_incarnation_id: runtime.control_incarnation_id().clone(),
            checkpoint_signing_authority_ref: authority_ref,
            checkpoint_signing_authority_sha256: checkpoint_authority
                .canonical_checkpoint_signing_authority_sha256()
                .clone(),
            initial_control_checkpoints: domains
                .iter()
                .zip(&references)
                .zip(&checkpoints)
                .map(|((domain, reference), record)| InitialControlCheckpointV1 {
                    checkpoint_domain_id: domain.clone(),
                    control_checkpoint_ref: reference.clone(),
                    control_checkpoint_sha256: record.canonical_control_checkpoint_sha256().clone(),
                    domain_epoch: ControlDomainEpochV1::new(1).unwrap(),
                    domain_sequence: ControlDomainSequenceV1::new(1).unwrap(),
                })
                .collect(),
            committed_at: UnixTimeV1::new(20).unwrap(),
            deployment_runtime_authority_ref: resident_ref("runtime-1"),
            deployment_runtime_authority_sha256: runtime
                .canonical_deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: runtime.principal_id().clone(),
            issuer_key_id: runtime.subject_key_id().clone(),
            issuer_key_epoch: *runtime.subject_key_epoch(),
        };
        let signing = SigningAuthorityBindingV1::integrated_runtime(
            value.issuer_principal_id.clone(),
            value.issuer_key_id.clone(),
            value.issuer_key_epoch,
            IntegratedRuntimeAuthorityV1 {
                deployment_runtime_authority_ref: value.deployment_runtime_authority_ref.clone(),
                deployment_runtime_authority_sha256: ArtifactDigest::parse(
                    value.deployment_runtime_authority_sha256.as_str(),
                )
                .unwrap(),
                deployment_binding_ref: binding_ref,
                deployment_binding_sha256: ArtifactDigest::parse(
                    value.deployment_binding_sha256.as_str(),
                )
                .unwrap(),
                checked_principal_trust_checkpoint_ref: resident_ref("trust-1"),
                checked_principal_trust_checkpoint_sha256: digest!(ArtifactDigest, 'd'),
            },
        )
        .unwrap();
        Self {
            binding,
            runtime,
            checkpoint_authority,
            checkpoints,
            checkpoint_values,
            runtime_value,
            references,
            domains,
            signing,
            value,
        }
    }

    fn admit<F>(
        &self,
        value: DeploymentControlStoreGenesisReceiptPreimageV1,
        verify: F,
    ) -> Result<DeploymentControlStoreGenesisReceiptV1, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &Ed25519PublicKeyV1,
            &[u8],
            &DeploymentControlStoreGenesisReceiptPreimageV1,
            &DeploymentControlStoreGenesisReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let candidate = DeploymentControlStoreGenesisReceiptV1::seal(value, signature())?;
        let checkpoints: Vec<_> = self.references.iter().zip(&self.checkpoints).collect();
        DeploymentControlStoreGenesisReceiptV1::admit_contextual_wire(
            wire(&candidate),
            &DeploymentControlStoreGenesisAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &self.value.schema_sha256,
                    signing_authority: &self.signing,
                },
                deployment_binding_ref: &self.value.deployment_binding_ref,
                deployment_binding: &self.binding,
                runtime_authority: &self.runtime,
                checkpoint_signing_authority_ref: &self.value.checkpoint_signing_authority_ref,
                checkpoint_signing_authority: &self.checkpoint_authority,
                required_checkpoint_domains: &self.domains,
                initial_control_checkpoints: &checkpoints,
            },
            verify,
        )
    }
}

#[test]
fn genesis_issuance_validates_before_using_the_runtime_signer() {
    let fixture = Fixture::new();
    let checkpoints: Vec<_> = fixture
        .references
        .iter()
        .zip(&fixture.checkpoints)
        .collect();
    let context = DeploymentControlStoreGenesisAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &fixture.value.schema_sha256,
            signing_authority: &fixture.signing,
        },
        deployment_binding_ref: &fixture.value.deployment_binding_ref,
        deployment_binding: &fixture.binding,
        runtime_authority: &fixture.runtime,
        checkpoint_signing_authority_ref: &fixture.value.checkpoint_signing_authority_ref,
        checkpoint_signing_authority: &fixture.checkpoint_authority,
        required_checkpoint_domains: &fixture.domains,
        initial_control_checkpoints: &checkpoints,
    };
    let calls = std::cell::Cell::new(0);
    let issued = DeploymentControlStoreGenesisReceiptV1::issue_contextual(
        fixture.value.clone(),
        &context,
        |domain, value, digest| {
            calls.set(calls.get() + 1);
            assert_eq!(
                domain,
                DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SIGNATURE_DOMAIN
            );
            assert_eq!(value, &fixture.value);
            assert_eq!(
                digest,
                &DeploymentControlStoreGenesisReceiptDigest::hash_canonical(value).unwrap()
            );
            Ok(signature())
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(issued.genesis_id(), &fixture.value.genesis_id);

    let mut invalid = fixture.value.clone();
    invalid.workspace_id = WorkspaceId::parse("other-workspace").unwrap();
    assert!(DeploymentControlStoreGenesisReceiptV1::issue_contextual(
        invalid,
        &context,
        |_, _, _| panic!("invalid candidate reached signer"),
    )
    .is_err());
}

#[test]
fn genesis_receipt_uses_runtime_key_and_preserves_exact_checkpoint_set() {
    let fixture = Fixture::new();
    assert_ne!(
        fixture.runtime.subject_key_id(),
        fixture.checkpoint_authority.subject_key_id()
    );
    assert_ne!(
        fixture.runtime.subject_public_key(),
        fixture.checkpoint_authority.subject_public_key()
    );
    let calls = std::cell::Cell::new(0);
    let admitted = fixture
        .admit(
            fixture.value.clone(),
            |public, domain, value, digest, sig| {
                calls.set(calls.get() + 1);
                assert_eq!(public, fixture.runtime.subject_public_key());
                assert_eq!(
                    domain,
                    DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SIGNATURE_DOMAIN
                );
                assert_eq!(value, &fixture.value);
                assert_eq!(
                    digest,
                    &DeploymentControlStoreGenesisReceiptDigest::hash_canonical(value).unwrap()
                );
                assert_eq!(sig, &signature());
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        admitted.initial_control_checkpoints(),
        &fixture.value.initial_control_checkpoints
    );
    assert_eq!(admitted.committed_at(), &fixture.value.committed_at);
    assert!(fixture
        .admit(fixture.value.clone(), |_, _, _, _, _| Err(denied(
            "genesis",
            "signature rejected"
        )))
        .is_err());
}

#[test]
fn genesis_rejects_mismatched_scope_runtime_and_checkpoint_coordinates_before_signature() {
    let fixture = Fixture::new();
    for case in 0..13 {
        let mut value = fixture.value.clone();
        match case {
            0 => value.schema_sha256 = schema_digest('9'),
            1 => value.workspace_id = WorkspaceId::parse("other-workspace").unwrap(),
            2 => value.deployment_binding_ref = resident_ref("other-binding"),
            3 => value.control_store_id = ControlStoreIdV1::parse("other-store").unwrap(),
            4 => {
                value.control_incarnation_id =
                    ControlIncarnationIdV1::parse("other-incarnation").unwrap()
            }
            5 => value.deployment_runtime_authority_ref = resident_ref("other-runtime"),
            6 => {
                value.deployment_runtime_authority_sha256 =
                    digest!(DeploymentRuntimeAuthorityDigest, 'f')
            }
            7 => value.issuer_key_id = fixture.checkpoint_authority.subject_key_id().clone(),
            8 => value.checkpoint_signing_authority_ref = resident_ref("other-signer"),
            9 => {
                value.checkpoint_signing_authority_sha256 =
                    digest!(CheckpointSigningAuthorityDigest, 'f')
            }
            10 => value.committed_at = UnixTimeV1::new(15).unwrap(),
            11 => value.committed_at = UnixTimeV1::new(24).unwrap(),
            12 => {
                value.initial_control_checkpoints[0].control_checkpoint_sha256 =
                    digest!(ControlStateCheckpointDigest, 'f')
            }
            _ => unreachable!(),
        }
        assert!(
            fixture
                .admit(value, |_, _, _, _, _| panic!(
                    "case {case} reached signature"
                ))
                .is_err(),
            "case {case}"
        );
    }
}

#[test]
fn genesis_requires_complete_ordered_initial_set_from_one_transaction() {
    let fixture = Fixture::new();
    for case in 0..5 {
        let mut value = fixture.value.clone();
        match case {
            0 => value.initial_control_checkpoints.clear(),
            1 => {
                value.initial_control_checkpoints.pop();
            }
            2 => value.initial_control_checkpoints.swap(0, 1),
            3 => {
                value.initial_control_checkpoints[1].control_checkpoint_ref = value
                    .initial_control_checkpoints[0]
                    .control_checkpoint_ref
                    .clone()
            }
            4 => {
                value.initial_control_checkpoints[0].domain_sequence =
                    ControlDomainSequenceV1::new(2).unwrap()
            }
            _ => unreachable!(),
        }
        assert!(fixture
            .admit(value, |_, _, _, _, _| panic!(
                "invalid set reached signature"
            ))
            .is_err());
    }
    let mut other = Fixture::new();
    let mut preimage = other.checkpoint_values[1].clone();
    preimage.committed_transaction_sequence = ControlTransactionSequenceV1::new(10).unwrap();
    other.checkpoints[1] = ControlStateCheckpointV1::seal(preimage, signature()).unwrap();
    other.value.initial_control_checkpoints[1].control_checkpoint_sha256 = other.checkpoints[1]
        .canonical_control_checkpoint_sha256()
        .clone();
    assert!(other
        .admit(other.value.clone(), |_, _, _, _, _| panic!(
            "different transactions reached signature"
        ))
        .is_err());
}

#[test]
fn genesis_rejects_incomplete_required_domains_and_unlike_admitted_checkpoints() {
    for case in 0..9 {
        let mut fixture = Fixture::new();
        match case {
            0 => fixture.domains.clear(),
            1 => fixture.domains.swap(0, 1),
            2 => fixture
                .domains
                .push(ControlCheckpointDomainIdV1::parse("routes").unwrap()),
            3 => {
                fixture.domains.pop();
            }
            4 => fixture.references[1] = fixture.references[0].clone(),
            5..=8 => {
                let mut value = fixture.checkpoint_values[1].clone();
                match case {
                    5 => value.checkpoint_profile_ref = resident_ref("other-profile"),
                    6 => value.issuer_key_id = fixture.runtime.subject_key_id().clone(),
                    7 => {
                        value.domain_sequence = ControlDomainSequenceV1::new(2).unwrap();
                        value.previous_control_checkpoint_ref =
                            Some(resident_ref("previous-checkpoint"));
                        value.previous_control_checkpoint_sha256 =
                            Some(digest!(ControlStateCheckpointDigest, 'f'));
                    }
                    8 => value.issued_at = UnixTimeV1::new(21).unwrap(),
                    _ => unreachable!(),
                }
                fixture.checkpoints[1] =
                    ControlStateCheckpointV1::seal(value, signature()).unwrap();
                fixture.value.initial_control_checkpoints[1].control_checkpoint_sha256 = fixture
                    .checkpoints[1]
                    .canonical_control_checkpoint_sha256()
                    .clone();
            }
            _ => unreachable!(),
        }
        assert!(
            fixture
                .admit(fixture.value.clone(), |_, _, _, _, _| panic!(
                    "case {case} reached signature"
                ))
                .is_err(),
            "case {case}"
        );
    }
}

#[test]
fn genesis_checks_supplied_runtime_state_allowlist_and_bytes_but_does_not_discover_current_heads() {
    for case in 0..3 {
        let mut fixture = Fixture::new();
        match case {
            0 => fixture.runtime_value.state = PrincipalAuthorityStateV1::Retired,
            1 => {
                fixture.runtime_value.permitted_record_schema_ids =
                    vec![schema_id("pale/effect-receipt.v1")]
            }
            2 => fixture.runtime_value.expires_at = UnixTimeV1::new(19).unwrap(),
            _ => unreachable!(),
        }
        fixture.runtime =
            DeploymentRuntimeAuthorityV1::seal(fixture.runtime_value.clone(), signature()).unwrap();
        fixture.value.deployment_runtime_authority_sha256 = fixture
            .runtime
            .canonical_deployment_runtime_authority_sha256()
            .clone();
        fixture.signing = SigningAuthorityBindingV1::integrated_runtime(
            fixture.value.issuer_principal_id.clone(),
            fixture.value.issuer_key_id.clone(),
            fixture.value.issuer_key_epoch,
            IntegratedRuntimeAuthorityV1 {
                deployment_runtime_authority_ref: fixture
                    .value
                    .deployment_runtime_authority_ref
                    .clone(),
                deployment_runtime_authority_sha256: ArtifactDigest::parse(
                    fixture.value.deployment_runtime_authority_sha256.as_str(),
                )
                .unwrap(),
                deployment_binding_ref: fixture.value.deployment_binding_ref.clone(),
                deployment_binding_sha256: ArtifactDigest::parse(
                    fixture.value.deployment_binding_sha256.as_str(),
                )
                .unwrap(),
                checked_principal_trust_checkpoint_ref: resident_ref("caller-resolved-trust"),
                checked_principal_trust_checkpoint_sha256: digest!(ArtifactDigest, 'd'),
            },
        )
        .unwrap();
        assert!(fixture
            .admit(fixture.value.clone(), |_, _, _, _, _| panic!(
                "invalid runtime reached signature"
            ))
            .is_err());
    }
    // Successful contextual admission in the first test is deliberately not a
    // current-head/anchor proof: this pure owner performs no native store reads.
    let fixture = Fixture::new();
    let mut value = serde_json::to_value(
        DeploymentControlStoreGenesisReceiptV1::seal(fixture.value.clone(), signature()).unwrap(),
    )
    .unwrap();
    value["committed_at"] = serde_json::json!(21);
    let candidate =
        serde_json::from_value::<DeploymentControlStoreGenesisReceiptWireV1>(value).unwrap();
    let checkpoints: Vec<_> = fixture
        .references
        .iter()
        .zip(&fixture.checkpoints)
        .collect();
    assert!(
        DeploymentControlStoreGenesisReceiptV1::admit_contextual_wire(
            candidate,
            &DeploymentControlStoreGenesisAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &fixture.value.schema_sha256,
                    signing_authority: &fixture.signing
                },
                deployment_binding_ref: &fixture.value.deployment_binding_ref,
                deployment_binding: &fixture.binding,
                runtime_authority: &fixture.runtime,
                checkpoint_signing_authority_ref: &fixture.value.checkpoint_signing_authority_ref,
                checkpoint_signing_authority: &fixture.checkpoint_authority,
                required_checkpoint_domains: &fixture.domains,
                initial_control_checkpoints: &checkpoints,
            },
            |_, _, _, _, _| panic!("changed signed bytes reached verifier")
        )
        .is_err()
    );
}

#[test]
fn genesis_wire_rejects_unknown_initial_checkpoint_member_fields() {
    let fixture = Fixture::new();
    let mut wire_value = serde_json::to_value(
        DeploymentControlStoreGenesisReceiptV1::seal(fixture.value, signature()).unwrap(),
    )
    .unwrap();
    wire_value["initial_control_checkpoints"][0]["unrecognized"] = serde_json::json!(true);
    assert!(
        serde_json::from_value::<DeploymentControlStoreGenesisReceiptWireV1>(wire_value).is_err()
    );
}
