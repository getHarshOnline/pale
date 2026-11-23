//! Submission authentication and exact head/candidate representation laws.

use super::*;
use std::cell::Cell;

fn authority() -> LocalAdminAuthorityV1 {
    authority_with_state(PrincipalAuthorityStateV1::Active)
}

fn authority_with_state(state: PrincipalAuthorityStateV1) -> LocalAdminAuthorityV1 {
    LocalAdminAuthorityV1::seal(
        LocalAdminAuthorityPreimageV1 {
            schema_id: SchemaId::parse(LOCAL_ADMIN_AUTHORITY_V1_SCHEMA).unwrap(),
            schema_sha256: schema_digest('a'),
            admin_authority_id: LocalAdminAuthorityIdV1::parse("administrator").unwrap(),
            principal_id: principal(),
            principal_record_ref: record_ref("principal-record-1"),
            principal_record_sha256: PrincipalRecordDigest::parse(sha('b')).unwrap(),
            recovery_policy_ref: record_ref("recovery-policy-1"),
            recovery_policy_sha256: PrincipalRecoveryPolicyDigest::parse(sha('c')).unwrap(),
            subject_identity: LocalAdminSubjectIdentityV1::parse("operator").unwrap(),
            subject_key_id: ResidentKeyIdV1::parse("operator-key").unwrap(),
            subject_key_epoch: authority_key_epoch(),
            subject_public_key: Ed25519PublicKeyV1::parse("A".repeat(43)).unwrap(),
            allowed_operations: vec![LocalAdminOperationV1::FinalizeDeployment],
            state,
            issued_at: UnixTimeV1::new(10).unwrap(),
            expires_at: UnixTimeV1::new(30).unwrap(),
            authority_epoch: LocalAdminAuthorityEpochV1::new(1).unwrap(),
            previous_authority_ref: None,
            previous_authority_sha256: None,
            issuer_principal_id: principal(),
            issuer_key_id: issuer_key(),
            issuer_key_epoch: key_epoch(),
        },
        signature(),
    )
    .unwrap()
}

fn trust(authority: &LocalAdminAuthorityV1) -> PrincipalTrustCheckpointV1 {
    let mut value = super::trust_checkpoint::trust_preimage(1, None);
    value.local_admin_authority_sha256 = authority.canonical_admin_authority_sha256().clone();
    PrincipalTrustCheckpointV1::seal(value, signature()).unwrap()
}

fn command(authority: &LocalAdminAuthorityV1) -> LocalAdminCommandPreimageV1 {
    let candidate_records = [
        DEPLOYMENT_BINDING_V1_SCHEMA,
        CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
        DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
        PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, schema)| LocalAdminCandidateRecordV1 {
        candidate_record_schema_id: SchemaId::parse(schema).unwrap(),
        candidate_record_ref: record_ref(&format!("candidate-{index}")),
        candidate_record_sha256: artifact('b'),
    })
    .collect::<Vec<_>>();
    LocalAdminCommandPreimageV1 {
        schema_id: SchemaId::parse(LOCAL_ADMIN_COMMAND_V1_SCHEMA).unwrap(),
        schema_sha256: schema_digest('d'),
        admin_command_id: LocalAdminCommandIdV1::parse("command-1").unwrap(),
        operation: LocalAdminOperationV1::FinalizeDeployment,
        admin_authority_ref: record_ref("admin-authority-1"),
        admin_authority_sha256: authority.canonical_admin_authority_sha256().clone(),
        candidate_set_sha256: LocalAdminCandidateSetDigest::hash_canonical(&candidate_records)
            .unwrap(),
        candidate_records,
        expected_heads: vec![LocalAdminExpectedHeadV1 {
            head_kind: LocalAdminHeadSelectorV1::PrincipalTrustCheckpoint {
                principal_id: principal(),
                trust_store_id: ControlStoreIdV1::parse("trust-store-1").unwrap(),
                trust_store_incarnation_id: ControlIncarnationIdV1::parse("trust-incarnation-1")
                    .unwrap(),
            },
            expected_head_ref: Some(record_ref("current-trust")),
            expected_head_sha256: Some(artifact('c')),
            expected_head_epoch: Some(LocalAdminHeadEpochV1::new(1).unwrap()),
        }],
        idempotency_key: LocalAdminIdempotencyKeyV1::parse("finalization-1").unwrap(),
        issued_at: UnixTimeV1::new(12).unwrap(),
        expires_at: UnixTimeV1::new(25).unwrap(),
        signer_identity: authority.subject_identity().clone(),
        signer_key_id: authority.subject_key_id().clone(),
        signer_key_epoch: *authority.subject_key_epoch(),
    }
}

fn command_wire(value: LocalAdminCommandPreimageV1) -> LocalAdminCommandWireV1 {
    wire_from(&LocalAdminCommandV1::seal(value, signature()).unwrap())
}

#[test]
fn authenticated_command_uses_current_delegated_subject_and_exact_domain() {
    let authority = authority();
    let trust = trust(&authority);
    let reference = record_ref("admin-authority-1");
    let schema = schema_digest('d');
    let context = LocalAdminCommandAdmissionContextV1 {
        expected_schema_sha256: &schema,
        trust_checkpoint: &trust,
        admin_authority_ref: &reference,
        admin_authority: &authority,
        now: UnixTimeV1::new(20).unwrap(),
    };
    let calls = Cell::new(0);
    let original = command(&authority);
    let admitted = LocalAdminCommandV1::admit_authenticated_wire(
        command_wire(original.clone()),
        &context,
        |key, domain, value, digest, sig| {
            calls.set(calls.get() + 1);
            assert_eq!(key, authority.subject_public_key());
            assert_eq!(domain, b"PALE_LOCAL_ADMIN_COMMAND_V1\0");
            assert_eq!(value, &original);
            assert_eq!(
                *digest,
                LocalAdminCommandDigest::hash_canonical(value).unwrap()
            );
            assert_eq!(sig, &signature());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(admitted.candidate_records(), &original.candidate_records);
    assert_eq!(admitted.expected_heads(), &original.expected_heads);
    assert!(LocalAdminCommandV1::admit_authenticated_wire(
        command_wire(original),
        &context,
        |_, _, _, _, _| reject_test_signature()
    )
    .is_err());
}

#[test]
fn historical_command_uses_receipt_commit_time_without_a_current_clock() {
    let authority = authority();
    let trust = trust(&authority);
    let reference = record_ref("admin-authority-1");
    let schema = schema_digest('d');
    let original = command(&authority);
    let context = HistoricalLocalAdminCommandAdmissionContextV1 {
        expected_schema_sha256: &schema,
        trust_checkpoint: &trust,
        admin_authority_ref: &reference,
        admin_authority: &authority,
        committed_at: UnixTimeV1::new(20).unwrap(),
    };
    let calls = Cell::new(0);
    let admitted = LocalAdminCommandV1::admit_historical_authenticated_wire(
        command_wire(original.clone()),
        &context,
        |key, domain, value, digest, sig| {
            calls.set(calls.get() + 1);
            assert_eq!(key, authority.subject_public_key());
            assert_eq!(domain, LOCAL_ADMIN_COMMAND_V1_SIGNATURE_DOMAIN);
            assert_eq!(value, &original);
            assert_eq!(
                digest,
                &LocalAdminCommandDigest::hash_canonical(value).unwrap()
            );
            assert_eq!(sig, &signature());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(admitted.admin_command_id(), &original.admin_command_id);
    assert!(LocalAdminCommandV1::admit_historical_authenticated_wire(
        command_wire(original),
        &context,
        |_, _, _, _, _| reject_test_signature(),
    )
    .is_err());
}

#[test]
fn historical_command_commit_time_uses_inclusive_start_and_exclusive_end() {
    let authority = authority();
    let trust = trust(&authority);
    let reference = record_ref("admin-authority-1");
    let schema = schema_digest('d');
    let value = command(&authority);
    for (committed_at, accepted) in [(12, true), (11, false), (24, true), (25, false)] {
        let context = HistoricalLocalAdminCommandAdmissionContextV1 {
            expected_schema_sha256: &schema,
            trust_checkpoint: &trust,
            admin_authority_ref: &reference,
            admin_authority: &authority,
            committed_at: UnixTimeV1::new(committed_at).unwrap(),
        };
        let calls = Cell::new(0);
        let result = LocalAdminCommandV1::admit_historical_authenticated_wire(
            command_wire(value.clone()),
            &context,
            |_, _, _, _, _| {
                calls.set(calls.get() + 1);
                Ok(())
            },
        );
        assert_eq!(result.is_ok(), accepted, "commit time {committed_at}");
        assert_eq!(calls.get(), usize::from(accepted));
    }
}

#[test]
fn historical_command_reuses_exact_authority_checks_before_signature() {
    for case in 0..14 {
        let authority = authority_with_state(if case == 13 {
            PrincipalAuthorityStateV1::Retired
        } else {
            PrincipalAuthorityStateV1::Active
        });
        let mut trust_value = super::trust_checkpoint::trust_preimage(1, None);
        trust_value.local_admin_authority_sha256 =
            authority.canonical_admin_authority_sha256().clone();
        let mut reference = record_ref("admin-authority-1");
        let mut value = command(&authority);
        match case {
            0 => value.schema_sha256 = schema_digest('e'),
            1 => value.admin_authority_ref = record_ref("foreign-authority"),
            2 => value.admin_authority_sha256 = LocalAdminAuthorityDigest::parse(sha('f')).unwrap(),
            3 => value.operation = LocalAdminOperationV1::RotatePrincipalKey,
            4 => value.signer_identity = LocalAdminSubjectIdentityV1::parse("foreign").unwrap(),
            5 => value.signer_key_id = ResidentKeyIdV1::parse("foreign-key").unwrap(),
            6 => value.signer_key_epoch = SigningKeyEpochV1::new(2).unwrap(),
            7 => value.issued_at = UnixTimeV1::new(9).unwrap(),
            8 => value.expires_at = UnixTimeV1::new(31).unwrap(),
            9 => trust_value.local_admin_authority_ref = record_ref("different-admin"),
            10 => {
                trust_value.local_admin_authority_sha256 =
                    LocalAdminAuthorityDigest::parse(sha('f')).unwrap()
            }
            11 => {
                trust_value.principal_id = ResidentPrincipalIdV1::parse("other-principal").unwrap();
                trust_value.issuer_principal_id = trust_value.principal_id.clone();
            }
            12 => {
                reference = record_ref("different-resolved-admin");
                value.admin_authority_ref = reference.clone();
            }
            13 => {}
            _ => unreachable!(),
        }
        let trust = PrincipalTrustCheckpointV1::seal(trust_value, signature()).unwrap();
        let schema = schema_digest('d');
        let context = HistoricalLocalAdminCommandAdmissionContextV1 {
            expected_schema_sha256: &schema,
            trust_checkpoint: &trust,
            admin_authority_ref: &reference,
            admin_authority: &authority,
            committed_at: UnixTimeV1::new(20).unwrap(),
        };
        assert!(
            LocalAdminCommandV1::admit_historical_authenticated_wire(
                command_wire(value),
                &context,
                |_, _, _, _, _| panic!("invalid historical context {case} reached signature"),
            )
            .is_err(),
            "case {case}"
        );
    }
}

#[test]
fn command_context_rejects_unlike_authority_subject_time_and_permission_before_signature() {
    let authority = authority();
    let trust = trust(&authority);
    let reference = record_ref("admin-authority-1");
    let schema = schema_digest('d');
    for case in 0..10 {
        let mut value = command(&authority);
        match case {
            0 => value.schema_sha256 = schema_digest('e'),
            1 => value.admin_authority_ref = record_ref("foreign-authority"),
            2 => value.admin_authority_sha256 = LocalAdminAuthorityDigest::parse(sha('f')).unwrap(),
            3 => value.operation = LocalAdminOperationV1::RotatePrincipalKey,
            4 => value.signer_identity = LocalAdminSubjectIdentityV1::parse("foreign").unwrap(),
            5 => value.signer_key_id = ResidentKeyIdV1::parse("root-is-not-subject").unwrap(),
            6 => value.signer_key_epoch = SigningKeyEpochV1::new(2).unwrap(),
            7 => value.issued_at = UnixTimeV1::new(9).unwrap(),
            8 => value.expires_at = UnixTimeV1::new(31).unwrap(),
            9 => value.expires_at = UnixTimeV1::new(20).unwrap(),
            _ => unreachable!(),
        }
        let context = LocalAdminCommandAdmissionContextV1 {
            expected_schema_sha256: &schema,
            trust_checkpoint: &trust,
            admin_authority_ref: &reference,
            admin_authority: &authority,
            now: UnixTimeV1::new(20).unwrap(),
        };
        let result = LocalAdminCommandV1::admit_authenticated_wire(
            command_wire(value),
            &context,
            |_, _, _, _, _| panic!("invalid context reached signer: {case}"),
        );
        assert!(
            matches!(
                result,
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "local administration command",
                    invariant: "current delegated subject, operation and validity interval",
                })
            ),
            "case {case}"
        );
    }
}

#[test]
fn command_rejects_unlike_current_trust_head_and_retired_authority_before_signature() {
    for case in 0..5 {
        let authority = authority_with_state(if case == 4 {
            PrincipalAuthorityStateV1::Retired
        } else {
            PrincipalAuthorityStateV1::Active
        });
        let mut trust_value = super::trust_checkpoint::trust_preimage(1, None);
        trust_value.local_admin_authority_sha256 =
            authority.canonical_admin_authority_sha256().clone();
        let mut reference = record_ref("admin-authority-1");
        match case {
            0 => trust_value.local_admin_authority_ref = record_ref("different-current-admin"),
            1 => {
                trust_value.local_admin_authority_sha256 =
                    LocalAdminAuthorityDigest::parse(sha('f')).unwrap()
            }
            2 => {
                trust_value.principal_id = ResidentPrincipalIdV1::parse("other-principal").unwrap();
                trust_value.issuer_principal_id = trust_value.principal_id.clone();
            }
            3 => reference = record_ref("different-resolved-admin"),
            4 => {}
            _ => unreachable!(),
        }
        let trust = PrincipalTrustCheckpointV1::seal(trust_value, signature()).unwrap();
        let mut value = command(&authority);
        value.admin_authority_ref = reference.clone();
        let schema = schema_digest('d');
        let context = LocalAdminCommandAdmissionContextV1 {
            expected_schema_sha256: &schema,
            trust_checkpoint: &trust,
            admin_authority_ref: &reference,
            admin_authority: &authority,
            now: UnixTimeV1::new(20).unwrap(),
        };
        assert!(
            LocalAdminCommandV1::admit_authenticated_wire(
                command_wire(value),
                &context,
                |_, _, _, _, _| panic!("invalid current context {case} reached signature")
            )
            .is_err(),
            "case {case}"
        );
    }
}

#[test]
fn command_candidate_hash_covers_order_and_rejects_duplicate_or_system_records() {
    let authority = authority();
    let original = command(&authority);
    let mut reordered = original.clone();
    reordered.candidate_records.swap(0, 1);
    assert!(LocalAdminCommandV1::seal(reordered.clone(), signature()).is_err());
    reordered.candidate_set_sha256 =
        LocalAdminCandidateSetDigest::hash_canonical(&reordered.candidate_records).unwrap();
    let changed = LocalAdminCommandV1::seal(reordered, signature()).unwrap();
    let original_record = LocalAdminCommandV1::seal(original.clone(), signature()).unwrap();
    assert_ne!(
        changed.canonical_admin_command_sha256(),
        original_record.canonical_admin_command_sha256()
    );
    // Authentication preserves declared order; the candidate owner still rejects
    // an operation-incompatible array before execution.
    for system_record in [false, true] {
        let mut invalid = original.clone();
        if system_record {
            invalid.candidate_records[0].candidate_record_schema_id =
                SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA).unwrap();
        } else {
            invalid.candidate_records[1].candidate_record_ref =
                invalid.candidate_records[0].candidate_record_ref.clone();
        }
        invalid.candidate_set_sha256 =
            LocalAdminCandidateSetDigest::hash_canonical(&invalid.candidate_records).unwrap();
        assert!(LocalAdminCommandV1::seal(invalid, signature()).is_err());
    }
}

#[test]
fn expected_heads_preserve_absence_and_reject_scope_disguised_duplicate_slots() {
    let authority = authority();
    let original = command(&authority);
    for case in 0..4 {
        let mut value = original.clone();
        match case {
            0 => value.expected_heads[0].expected_head_ref = None,
            1 => {
                value.expected_heads[0].expected_head_ref = None;
                value.expected_heads[0].expected_head_sha256 = None;
            }
            2 => value.expected_heads.push(value.expected_heads[0].clone()),
            3 => {
                let mut other = value.expected_heads[0].clone();
                if let LocalAdminHeadSelectorV1::PrincipalTrustCheckpoint {
                    trust_store_id, ..
                } = &mut other.head_kind
                {
                    *trust_store_id = ControlStoreIdV1::parse("other-store").unwrap();
                }
                value.expected_heads.push(other);
            }
            _ => unreachable!(),
        }
        assert!(
            LocalAdminCommandV1::seal(value, signature()).is_err(),
            "case {case}"
        );
    }
    let mut absent = original;
    absent.expected_heads[0].expected_head_ref = None;
    absent.expected_heads[0].expected_head_sha256 = None;
    absent.expected_heads[0].expected_head_epoch = None;
    assert!(LocalAdminCommandV1::seal(absent, signature()).is_ok());

    let mut distinct = command(&authority);
    let mut head = distinct.expected_heads[0].clone();
    head.head_kind = LocalAdminHeadSelectorV1::CheckpointSigningAuthority {
        principal_id: principal(),
        checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1::parse("signer-a").unwrap(),
        control_store_id: ControlStoreIdV1::parse("store-a").unwrap(),
        control_incarnation_id: ControlIncarnationIdV1::parse("incarnation-a").unwrap(),
        scope: CheckpointSigningScopeV1::Deployment {
            deployment_binding_ref: record_ref("binding-a"),
            deployment_binding_sha256: DeploymentBindingDigest::parse(sha('a')).unwrap(),
        },
    };
    distinct.expected_heads.push(head.clone());
    if let LocalAdminHeadSelectorV1::CheckpointSigningAuthority {
        checkpoint_signing_authority_id,
        ..
    } = &mut head.head_kind
    {
        *checkpoint_signing_authority_id =
            CheckpointSigningAuthorityIdV1::parse("signer-b").unwrap();
    }
    distinct.expected_heads.push(head.clone());
    assert!(LocalAdminCommandV1::seal(distinct.clone(), signature()).is_ok());
    if let LocalAdminHeadSelectorV1::CheckpointSigningAuthority {
        control_store_id, ..
    } = &mut head.head_kind
    {
        *control_store_id = ControlStoreIdV1::parse("altered-scope").unwrap();
    }
    distinct.expected_heads.push(head);
    assert!(LocalAdminCommandV1::seal(distinct, signature()).is_err());

    for selector in [
        LocalAdminHeadSelectorV1::LocalRoute {
            route_owner_principal_id: principal(),
            route_id: LocalRouteIdV1::parse("route-a").unwrap(),
            workspace_id: ResidentWorkspaceIdV1::parse("workspace-a").unwrap(),
            deployment_binding_ref: record_ref("binding-a"),
            deployment_binding_sha256: DeploymentBindingDigest::parse(sha('a')).unwrap(),
        },
        LocalAdminHeadSelectorV1::ExecutionFence {
            execution_fence_id: ExecutionOwnerFenceIdV1::parse("fence-a").unwrap(),
            execution_path_id: ExecutionPathSelectionIdV1::parse("path-a").unwrap(),
            deployment_binding_ref: record_ref("binding-a"),
            deployment_binding_sha256: DeploymentBindingDigest::parse(sha('a')).unwrap(),
            principal_scope: principal(),
            controller_scope: ResidentControllerIdV1::parse("controller-a").unwrap(),
            relationship_scope: None,
        },
    ] {
        let encoded = serde_json::to_value(&selector).unwrap();
        let decoded: LocalAdminHeadSelectorV1 = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, selector);
        let mut missing = encoded;
        let field = match &selector {
            LocalAdminHeadSelectorV1::LocalRoute { .. } => "workspace_id",
            LocalAdminHeadSelectorV1::ExecutionFence { .. } => "execution_path_id",
            _ => unreachable!(),
        };
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<LocalAdminHeadSelectorV1>(missing).is_err());
        let mut value = command(&authority);
        value.expected_heads[0].head_kind = selector;
        let original = LocalAdminCommandV1::seal(value.clone(), signature()).unwrap();
        let mut other = value.expected_heads[0].clone();
        match &mut other.head_kind {
            LocalAdminHeadSelectorV1::LocalRoute { workspace_id, .. } => {
                *workspace_id = ResidentWorkspaceIdV1::parse("altered-workspace").unwrap();
            }
            LocalAdminHeadSelectorV1::ExecutionFence {
                execution_path_id, ..
            } => {
                *execution_path_id = ExecutionPathSelectionIdV1::parse("altered-path").unwrap();
            }
            _ => unreachable!(),
        }
        // The complete scope is signed, but changing it cannot invent another
        // slot beside the original route/fence in the same expected-head set.
        let mut altered = value.clone();
        altered.expected_heads[0] = other.clone();
        let changed = LocalAdminCommandV1::seal(altered, signature()).unwrap();
        assert_ne!(
            original.canonical_admin_command_sha256(),
            changed.canonical_admin_command_sha256()
        );
        value.expected_heads.push(other);
        assert!(LocalAdminCommandV1::seal(value, signature()).is_err());
    }
}

#[test]
fn command_wire_is_closed_and_comparison_epoch_uses_json_safe_bounds() {
    let record = LocalAdminCommandV1::seal(command(&authority()), signature()).unwrap();
    let mut value = serde_json::to_value(record).unwrap();
    value["expected_heads"][0]["head_kind"]["kind"] = serde_json::json!("universal-head");
    assert!(serde_json::from_value::<LocalAdminCommandWireV1>(value).is_err());
    assert!(LocalAdminHeadEpochV1::new(0).is_err());
    assert!(LocalAdminHeadEpochV1::new(crate::canon::JSON_SAFE_INTEGER_MAX).is_ok());
    assert!(LocalAdminHeadEpochV1::new(crate::canon::JSON_SAFE_INTEGER_MAX + 1).is_err());
}
