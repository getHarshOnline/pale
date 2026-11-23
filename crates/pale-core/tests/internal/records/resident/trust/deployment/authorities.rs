//! Deployment signer admission and canonical authority-head commitments.

use super::*;

fn checkpoint_signer_preimage(
    binding_ref: ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
) -> CheckpointSigningAuthorityPreimageV1 {
    let runtime = runtime_preimage(binding_ref.clone(), binding);
    CheckpointSigningAuthorityPreimageV1 {
        schema_id: schema_id(CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA),
        schema_sha256: schema_digest('7'),
        checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1::parse("signer-1")
            .expect("signer id"),
        principal_id: runtime.principal_id,
        control_store_id: runtime.control_store_id,
        control_incarnation_id: runtime.control_incarnation_id,
        scope: CheckpointSigningScopeV1::Deployment {
            deployment_binding_ref: binding_ref,
            deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
        },
        checkpoint_profile_ref: resident_ref("deployment-profile"),
        checkpoint_profile_sha256: digest!(CheckpointProfileDigest, 'a'),
        allowed_domain_prefixes: ["operation:", "route:"]
            .into_iter()
            .map(|prefix| CheckpointDomainPrefixV1::parse(prefix).expect("domain prefix"))
            .collect(),
        subject_key_id: key("checkpoint-key"),
        subject_key_epoch: runtime.subject_key_epoch,
        subject_public_key: runtime.subject_public_key,
        signer_epoch: CheckpointSignerEpochV1::new(1).expect("signer epoch"),
        state: PrincipalAuthorityStateV1::Active,
        issued_at: runtime.issued_at,
        expires_at: runtime.expires_at,
        previous_authority_ref: None,
        previous_authority_sha256: None,
        issuer_principal_id: runtime.issuer_principal_id,
        issuer_key_id: runtime.issuer_key_id,
        issuer_key_epoch: runtime.issuer_key_epoch,
    }
}

#[test]
fn deployment_checkpoint_signer_admits_only_exact_initial_scope_before_signature() {
    let proposal = proposal();
    let proposal_ref = resident_ref("proposal-ref-1");
    let acceptance = acceptance(&proposal_ref, &proposal);
    let acceptance_ref = resident_ref("acceptance-ref-1");
    let binding = binding(&proposal_ref, &proposal, &acceptance_ref, &acceptance);
    let binding_ref = resident_ref("binding-ref-1");
    let preimage = checkpoint_signer_preimage(binding_ref.clone(), &binding);
    let context = DeploymentCheckpointSigningAdmissionContextV1 {
        expected_schema_sha256: &preimage.schema_sha256,
        deployment_binding_ref: &binding_ref,
        deployment_binding: &binding,
        control_store_id: &preimage.control_store_id,
        control_incarnation_id: &preimage.control_incarnation_id,
        checkpoint_profile_ref: &preimage.checkpoint_profile_ref,
        checkpoint_profile_sha256: &preimage.checkpoint_profile_sha256,
        signer: PrincipalKeyAdmissionContextV1::new(
            &preimage.principal_id,
            &preimage.issuer_key_id,
            preimage.issuer_key_epoch,
        ),
    };
    let original =
        CheckpointSigningAuthorityV1::seal(preimage.clone(), signature()).expect("signer shape");
    let mut prefixes = 0;
    let admitted = CheckpointSigningAuthorityV1::admit_initial_deployment_wire(
        wire(&original),
        &context,
        |prefix| {
            assert_eq!(prefix.as_str(), ["operation:", "route:"][prefixes]);
            prefixes += 1;
            Ok(())
        },
        |domain, actual, digest, sig| {
            assert_eq!(domain, CHECKPOINT_SIGNING_AUTHORITY_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &preimage);
            assert_eq!(
                digest,
                original.canonical_checkpoint_signing_authority_sha256()
            );
            assert_eq!(sig, &signature());
            Ok(())
        },
    )
    .expect("exact owner callbacks");
    assert_eq!(prefixes, 2);
    assert_eq!(admitted, original);

    let mutations: [fn(&mut CheckpointSigningAuthorityPreimageV1); 13] = [
        |p| p.schema_sha256 = schema_digest('8'),
        |p| p.control_store_id = ControlStoreIdV1::parse("other-store").unwrap(),
        |p| p.control_incarnation_id = ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
        |p| p.checkpoint_profile_ref = resident_ref("other-profile"),
        |p| p.checkpoint_profile_sha256 = digest!(CheckpointProfileDigest, 'b'),
        |p| p.issuer_key_id = key("other-root"),
        |p| p.issuer_key_epoch = PrincipalKeyEpochV1::new(2).unwrap(),
        |p| p.state = PrincipalAuthorityStateV1::Retired,
        |p| {
            p.scope = CheckpointSigningScopeV1::PrincipalTrustStore {
                bootstrap_id: PrincipalBootstrapIdV1::parse("bootstrap-other").unwrap(),
            }
        },
        |p| {
            if let CheckpointSigningScopeV1::Deployment {
                deployment_binding_ref,
                ..
            } = &mut p.scope
            {
                *deployment_binding_ref = resident_ref("other-binding");
            }
        },
        |p| {
            p.principal_id = principal("other-principal");
            p.issuer_principal_id = p.principal_id.clone();
        },
        |p| {
            if let CheckpointSigningScopeV1::Deployment {
                deployment_binding_sha256,
                ..
            } = &mut p.scope
            {
                *deployment_binding_sha256 = digest!(DeploymentBindingDigest, 'b');
            }
        },
        |p| {
            p.signer_epoch = CheckpointSignerEpochV1::new(2).unwrap();
            p.previous_authority_ref = Some(resident_ref("prior-signer"));
            p.previous_authority_sha256 = Some(digest!(CheckpointSigningAuthorityDigest, 'b'));
        },
    ];
    for mutate in mutations {
        let mut changed = preimage.clone();
        mutate(&mut changed);
        let changed = CheckpointSigningAuthorityV1::seal(changed, signature())
            .expect("negative is structurally valid");
        assert!(CheckpointSigningAuthorityV1::admit_initial_deployment_wire(
            wire(&changed),
            &context,
            |_| panic!("scope before domain permission"),
            |_, _, _, _| panic!("scope before signature"),
        )
        .is_err());
    }
    assert!(CheckpointSigningAuthorityV1::admit_initial_deployment_wire(
        wire(&original),
        &context,
        |_| Err(denied("profile", "domain permission")),
        |_, _, _, _| panic!("permission before signature"),
    )
    .is_err());
    assert!(CheckpointSigningAuthorityV1::admit_initial_deployment_wire(
        wire(&original),
        &context,
        |prefix| {
            if prefix.as_str() == "route:" {
                Err(denied("profile", "last domain permission"))
            } else {
                Ok(())
            }
        },
        |_, _, _, _| panic!("every permission must precede signature"),
    )
    .is_err());
    assert!(CheckpointSigningAuthorityV1::admit_initial_deployment_wire(
        wire(&original),
        &context,
        |_| Ok(()),
        |_, _, _, _| Err(denied("checkpoint signer", "signature")),
    )
    .is_err());
}

#[test]
fn deployment_authority_head_roots_require_lineage_order_and_exact_full_fields() {
    let proposal = proposal();
    let proposal_ref = resident_ref("proposal-ref-1");
    let acceptance = acceptance(&proposal_ref, &proposal);
    let acceptance_ref = resident_ref("acceptance-ref-1");
    let binding = binding(&proposal_ref, &proposal, &acceptance_ref, &acceptance);
    let binding_ref = resident_ref("binding-ref-1");
    let signer = CheckpointSigningAuthorityV1::seal(
        checkpoint_signer_preimage(binding_ref.clone(), &binding),
        signature(),
    )
    .expect("signer");
    let signer_head = signer
        .active_head(resident_ref("signer-ref"))
        .expect("active signer");
    assert_eq!(
        checkpoint_signing_authority_heads_root_sha256(std::slice::from_ref(&signer_head)).unwrap(),
        CheckpointSigningAuthorityHeadsRootDigest::hash_canonical(std::slice::from_ref(
            &signer_head
        ))
        .unwrap()
    );
    let mut other_signer = signer_head.clone();
    other_signer.checkpoint_signing_authority_id =
        CheckpointSigningAuthorityIdV1::parse("signer-2").unwrap();
    assert!(checkpoint_signing_authority_heads_root_sha256(&[
        signer_head.clone(),
        other_signer.clone()
    ])
    .is_ok());
    assert!(
        checkpoint_signing_authority_heads_root_sha256(&[other_signer, signer_head.clone()])
            .is_err()
    );
    assert!(
        checkpoint_signing_authority_heads_root_sha256(&[signer_head.clone(), signer_head])
            .is_err()
    );

    let runtime =
        DeploymentRuntimeAuthorityV1::seal(runtime_preimage(binding_ref, &binding), signature())
            .unwrap();
    let head = runtime.active_head(resident_ref("runtime-ref")).unwrap();
    let root = deployment_runtime_authority_heads_root_sha256(std::slice::from_ref(&head)).unwrap();
    assert_eq!(
        root,
        DeploymentRuntimeAuthorityHeadsRootDigest::hash_canonical(std::slice::from_ref(&head))
            .unwrap()
    );
    let mut changed = head.clone();
    changed.authority_ref = resident_ref("other-runtime-ref");
    assert_ne!(
        root,
        deployment_runtime_authority_heads_root_sha256(&[changed.clone()]).unwrap()
    );
    assert!(deployment_runtime_authority_heads_root_sha256(&[head.clone(), changed]).is_err());
    let mut other = head.clone();
    other.runtime_authority_id =
        DeploymentRuntimeAuthorityIdV1::parse("runtime-authority-2").unwrap();
    assert!(deployment_runtime_authority_heads_root_sha256(&[head.clone(), other.clone()]).is_ok());
    assert!(deployment_runtime_authority_heads_root_sha256(&[other, head]).is_err());
    assert_eq!(
        deployment_runtime_authority_heads_root_sha256(&[]).unwrap(),
        DeploymentRuntimeAuthorityHeadsRootDigest::hash_canonical(&Vec::<
            DeploymentRuntimeAuthorityHeadV1,
        >::new())
        .unwrap()
    );
}

#[test]
fn authority_head_commitments_cover_every_member_field() {
    let proposal = proposal();
    let proposal_ref = resident_ref("proposal-ref-1");
    let acceptance = acceptance(&proposal_ref, &proposal);
    let binding = binding(
        &proposal_ref,
        &proposal,
        &resident_ref("acceptance-ref-1"),
        &acceptance,
    );
    let binding_ref = resident_ref("binding-ref-1");
    let signer = CheckpointSigningAuthorityV1::seal(
        checkpoint_signer_preimage(binding_ref.clone(), &binding),
        signature(),
    )
    .unwrap();
    let head = signer.active_head(resident_ref("signer-ref")).unwrap();
    assert_eq!(
        head.checkpoint_signing_authority_id,
        *signer.checkpoint_signing_authority_id()
    );
    assert_eq!(head.control_store_id, *signer.control_store_id());
    assert_eq!(
        head.control_incarnation_id,
        *signer.control_incarnation_id()
    );
    assert_eq!(head.scope, *signer.scope());
    assert_eq!(head.authority_ref, resident_ref("signer-ref"));
    assert_eq!(
        head.authority_sha256,
        *signer.canonical_checkpoint_signing_authority_sha256()
    );
    assert_eq!(head.signer_epoch, *signer.signer_epoch());
    let root = checkpoint_signing_authority_heads_root_sha256(std::slice::from_ref(&head)).unwrap();
    let mutations: [fn(&mut CheckpointSigningAuthorityHeadV1); 7] = [
        |h| {
            h.checkpoint_signing_authority_id =
                CheckpointSigningAuthorityIdV1::parse("other-signer").unwrap()
        },
        |h| h.control_store_id = ControlStoreIdV1::parse("other-store").unwrap(),
        |h| h.control_incarnation_id = ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
        |h| {
            h.scope = CheckpointSigningScopeV1::PrincipalTrustStore {
                bootstrap_id: PrincipalBootstrapIdV1::parse("bootstrap").unwrap(),
            }
        },
        |h| h.authority_ref = resident_ref("other-authority"),
        |h| h.authority_sha256 = digest!(CheckpointSigningAuthorityDigest, 'b'),
        |h| h.signer_epoch = CheckpointSignerEpochV1::new(2).unwrap(),
    ];
    for mutate in mutations {
        let mut changed = head.clone();
        mutate(&mut changed);
        assert_ne!(
            root,
            checkpoint_signing_authority_heads_root_sha256(&[changed]).unwrap()
        );
    }
    let runtime =
        DeploymentRuntimeAuthorityV1::seal(runtime_preimage(binding_ref, &binding), signature())
            .unwrap();
    let head = runtime.active_head(resident_ref("runtime-ref")).unwrap();
    assert_eq!(head.runtime_authority_id, *runtime.runtime_authority_id());
    assert_eq!(
        head.deployment_binding_sha256,
        *runtime.deployment_binding_sha256()
    );
    assert_eq!(head.control_store_id, *runtime.control_store_id());
    assert_eq!(
        head.control_incarnation_id,
        *runtime.control_incarnation_id()
    );
    assert_eq!(head.authority_ref, resident_ref("runtime-ref"));
    assert_eq!(
        head.authority_sha256,
        *runtime.canonical_deployment_runtime_authority_sha256()
    );
    assert_eq!(head.authority_epoch, *runtime.authority_epoch());
    let root = deployment_runtime_authority_heads_root_sha256(std::slice::from_ref(&head)).unwrap();
    let mutations: [fn(&mut DeploymentRuntimeAuthorityHeadV1); 7] = [
        |h| {
            h.runtime_authority_id = DeploymentRuntimeAuthorityIdV1::parse("other-runtime").unwrap()
        },
        |h| h.deployment_binding_sha256 = digest!(DeploymentBindingDigest, 'b'),
        |h| h.control_store_id = ControlStoreIdV1::parse("other-store").unwrap(),
        |h| h.control_incarnation_id = ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
        |h| h.authority_ref = resident_ref("other-authority"),
        |h| h.authority_sha256 = digest!(DeploymentRuntimeAuthorityDigest, 'b'),
        |h| h.authority_epoch = DeploymentRuntimeAuthorityEpochV1::new(2).unwrap(),
    ];
    for mutate in mutations {
        let mut changed = head.clone();
        mutate(&mut changed);
        assert_ne!(
            root,
            deployment_runtime_authority_heads_root_sha256(&[changed]).unwrap()
        );
    }
}

#[test]
fn authority_heads_reject_retired_records_and_bound_sorted_arrays() {
    let proposal = proposal();
    let proposal_ref = resident_ref("proposal-ref-1");
    let acceptance = acceptance(&proposal_ref, &proposal);
    let binding = binding(
        &proposal_ref,
        &proposal,
        &resident_ref("acceptance-ref-1"),
        &acceptance,
    );
    let binding_ref = resident_ref("binding-ref-1");
    let signer_value = checkpoint_signer_preimage(binding_ref.clone(), &binding);
    let runtime_value = runtime_preimage(binding_ref, &binding);
    let signer = CheckpointSigningAuthorityV1::seal(signer_value.clone(), signature()).unwrap();
    let runtime = DeploymentRuntimeAuthorityV1::seal(runtime_value.clone(), signature()).unwrap();
    let signer_head = signer.active_head(resident_ref("signer-ref")).unwrap();
    let runtime_head = runtime.active_head(resident_ref("runtime-ref")).unwrap();
    let mut retired_signer = signer_value;
    retired_signer.state = PrincipalAuthorityStateV1::Retired;
    assert!(
        CheckpointSigningAuthorityV1::seal(retired_signer, signature())
            .unwrap()
            .active_head(resident_ref("signer-ref"))
            .is_err()
    );
    let mut retired_runtime = runtime_value;
    retired_runtime.state = PrincipalAuthorityStateV1::Retired;
    assert!(
        DeploymentRuntimeAuthorityV1::seal(retired_runtime, signature())
            .unwrap()
            .active_head(resident_ref("runtime-ref"))
            .is_err()
    );

    let signer_heads: Vec<_> = (0..=MAX_RESIDENT_SET_MEMBERS)
        .map(|index| {
            let mut head = signer_head.clone();
            head.checkpoint_signing_authority_id =
                CheckpointSigningAuthorityIdV1::parse(format!("signer-{index:04}")).unwrap();
            head
        })
        .collect();
    let runtime_heads: Vec<_> = (0..=MAX_RESIDENT_SET_MEMBERS)
        .map(|index| {
            let mut head = runtime_head.clone();
            head.runtime_authority_id =
                DeploymentRuntimeAuthorityIdV1::parse(format!("runtime-{index:04}")).unwrap();
            head
        })
        .collect();
    assert!(checkpoint_signing_authority_heads_root_sha256(
        &signer_heads[..MAX_RESIDENT_SET_MEMBERS]
    )
    .is_ok());
    assert!(deployment_runtime_authority_heads_root_sha256(
        &runtime_heads[..MAX_RESIDENT_SET_MEMBERS]
    )
    .is_ok());
    assert!(matches!(
        checkpoint_signing_authority_heads_root_sha256(&signer_heads),
        Err(InvalidResidentRecordV1::BoundExceeded { .. })
    ));
    assert!(matches!(
        deployment_runtime_authority_heads_root_sha256(&runtime_heads),
        Err(InvalidResidentRecordV1::BoundExceeded { .. })
    ));
}
