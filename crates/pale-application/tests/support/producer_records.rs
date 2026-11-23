//! Shared producer record setup with controlled signatures for custody sequence tests.

use super::principal_trust_test_support as principal_trust_bootstrap;
use pale_core::records::*;

pub(crate) fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>()).expect("schema digest")
}

pub(crate) fn producer_preimage(
    authority_ref: &ResidentRecordRefV1,
    authority: &ProducerIncarnationAuthorityV1,
    epoch: char,
    allocated_at: u64,
) -> ProducerIncarnationPreimageV1 {
    ProducerIncarnationPreimageV1 {
        schema_id: SchemaId::parse(PRODUCER_INCARNATION_V1_SCHEMA).expect("producer schema"),
        schema_sha256: schema_digest('e'),
        incarnation_id: ProducerIncarnationIdV1::parse(format!("incarnation-{epoch}"))
            .expect("incarnation id"),
        origin_principal_id: authority.principal_id().clone(),
        producer_id: ProducerIdV1::parse("producer-1").expect("producer"),
        producer_epoch: ProducerEpochV1::parse(std::iter::repeat_n(epoch, 32).collect::<String>())
            .expect("producer epoch"),
        producer_implementation_ref: ResidentRecordRefV1::parse("implementation-1")
            .expect("implementation ref"),
        producer_implementation_sha256: ArtifactDigest::parse("f".repeat(64))
            .expect("implementation digest"),
        producer_incarnation_authority_ref: authority_ref.clone(),
        producer_incarnation_authority_sha256: authority
            .canonical_producer_authority_sha256()
            .clone(),
        allocated_at: UnixTimeV1::new(allocated_at).expect("allocation time"),
        previous_incarnation_ref: None,
        previous_incarnation_sha256: None,
        issuer_principal_id: authority.principal_id().clone(),
        issuer_key_id: authority.subject_key_id().clone(),
        issuer_key_epoch: *authority.subject_key_epoch(),
    }
}

pub(crate) fn admit_allocation(
    preimage: &ProducerIncarnationPreimageV1,
    authority_ref: &ResidentRecordRefV1,
    authority: &ProducerIncarnationAuthorityV1,
    head: &ProducerIncarnationHeadAdmissionStateV1<'_>,
) -> ProducerIncarnationV1 {
    let digest = ProducerIncarnationDigest::hash_canonical(preimage).expect("producer digest");
    let context = ProducerIncarnationAuthorityAdmissionContextV1::new(authority_ref, authority)
        .expect("producer authority context");
    let wire =
        principal_trust_bootstrap::signed_wire(preimage, "canonical_incarnation_sha256", &digest);
    if matches!(head, ProducerIncarnationHeadAdmissionStateV1::Unallocated) {
        ProducerIncarnationV1::admit_initial_wire(
            wire,
            head,
            &preimage.schema_sha256,
            &context,
            |_, _| Ok(()),
            |_, _| true,
            |_, _, _, _, _| Ok(()),
        )
    } else {
        ProducerIncarnationV1::admit_successor_wire(
            wire,
            head,
            &preimage.schema_sha256,
            &context,
            |_, _| Ok(()),
            |_, _| true,
            |_, _, _, _, _| Ok(()),
        )
    }
    .expect("producer allocation admits")
}

pub(crate) fn trust_successor(
    mut preimage: PrincipalTrustCheckpointPreimageV1,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &PrincipalTrustCheckpointV1,
    reference: &str,
    root: ProducerIncarnationHeadsRootDigest,
    mutate: impl FnOnce(&mut PrincipalTrustCheckpointPreimageV1),
) -> (ResidentRecordRefV1, PrincipalTrustCheckpointV1) {
    preimage.trust_checkpoint_id =
        PrincipalTrustCheckpointIdV1::parse(reference).expect("trust checkpoint id");
    preimage.checkpoint_epoch = predecessor
        .checkpoint_epoch()
        .checked_successor()
        .expect("trust epoch");
    preimage.previous_trust_checkpoint_ref = Some(predecessor_ref.clone());
    preimage.previous_trust_checkpoint_sha256 =
        Some(predecessor.canonical_trust_checkpoint_sha256().clone());
    preimage.producer_incarnation_heads_root_sha256 = root;
    preimage.issued_at = UnixTimeV1::new(predecessor.issued_at().get() + 1).expect("trust time");
    mutate(&mut preimage);
    let digest = PrincipalTrustCheckpointDigest::hash_canonical(&preimage).expect("trust digest");
    let record = PrincipalTrustCheckpointV1::admit_successor_wire(
        principal_trust_bootstrap::signed_wire(
            &preimage,
            "canonical_trust_checkpoint_sha256",
            &digest,
        ),
        predecessor_ref,
        predecessor,
        &preimage.schema_sha256,
        |_, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("trust successor admits");
    (
        ResidentRecordRefV1::parse(reference).expect("trust ref"),
        record,
    )
}

pub(crate) fn control_successor(
    mut preimage: ControlStateCheckpointPreimageV1,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ControlStateCheckpointV1,
    reference: &str,
) -> (ResidentRecordRefV1, ControlStateCheckpointV1) {
    preimage.control_checkpoint_id =
        ControlCheckpointIdV1::parse(reference).expect("control checkpoint id");
    preimage.domain_sequence = predecessor
        .domain_sequence()
        .checked_successor()
        .expect("domain sequence");
    preimage.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
    preimage.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    preimage.committed_transaction_sequence =
        ControlTransactionSequenceV1::new(predecessor.committed_transaction_sequence().get() + 1)
            .expect("transaction sequence");
    preimage.issued_at = UnixTimeV1::new(predecessor.issued_at().get() + 1).expect("control time");
    let digest = ControlStateCheckpointDigest::hash_canonical(&preimage).expect("control digest");
    let signer = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: &preimage.checkpoint_signing_authority_ref,
        authority_sha256: &preimage.checkpoint_signing_authority_sha256,
        principal_id: &preimage.issuer_principal_id,
        control_store_id: &preimage.control_store_id,
        control_incarnation_id: &preimage.control_incarnation_id,
        checkpoint_domain_id: &preimage.checkpoint_domain_id,
        checkpoint_profile_ref: &preimage.checkpoint_profile_ref,
        checkpoint_profile_sha256: &preimage.checkpoint_profile_sha256,
        subject_key_id: &preimage.issuer_key_id,
        subject_key_epoch: preimage.issuer_key_epoch,
    };
    let record = ControlStateCheckpointV1::admit_successor_wire(
        principal_trust_bootstrap::signed_wire(
            &preimage,
            "canonical_control_checkpoint_sha256",
            &digest,
        ),
        predecessor_ref,
        predecessor,
        &preimage.schema_sha256,
        &signer,
        |_, _, _, _| Ok(()),
    )
    .expect("control successor admits");
    (
        ResidentRecordRefV1::parse(reference).expect("control ref"),
        record,
    )
}
