//! Law tests for signed resident trust contracts.

use super::*;
use crate::records::*;

mod administration;
mod administration_receipt;
mod principal;
mod producer;
mod producer_heads;
mod trust_checkpoint;
mod writer;

#[test]
fn principal_key_epoch_has_one_json_safe_boundary() {
    let maximum = crate::canon::JSON_SAFE_INTEGER_MAX;
    assert!(PrincipalKeyEpochV1::new(0).is_err());
    assert_eq!(PrincipalKeyEpochV1::new(1).expect("minimum").get(), 1);
    assert_eq!(
        PrincipalKeyEpochV1::new(maximum).expect("maximum").get(),
        maximum
    );
    assert!(PrincipalKeyEpochV1::new(maximum + 1).is_err());
    assert!(serde_json::from_value::<PrincipalKeyEpochV1>(serde_json::json!(0)).is_err());
    assert!(serde_json::from_value::<PrincipalKeyEpochV1>(serde_json::json!(maximum + 1)).is_err());

    let schema = serde_json::to_value(schemars::schema_for!(PrincipalKeyEpochV1))
        .expect("principal key epoch schema serializes");
    assert_eq!(schema.pointer("/minimum"), Some(&serde_json::json!(1)));
    assert_eq!(
        schema.pointer("/maximum"),
        Some(&serde_json::json!(maximum))
    );
}

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(sha(byte)).expect("schema digest")
}

fn principal_trust_checkpoint_digest(byte: char) -> PrincipalTrustCheckpointDigest {
    PrincipalTrustCheckpointDigest::parse(sha(byte)).expect("principal trust checkpoint digest")
}

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("resident record reference")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

fn reject_test_signature() -> Result<(), InvalidResidentRecordV1> {
    Err(InvalidResidentRecordV1::ContextualAdmission {
        record: "test signature",
        invariant: "authenticated",
    })
}

fn principal() -> ResidentPrincipalIdV1 {
    ResidentPrincipalIdV1::parse("principal-1").expect("principal")
}

fn issuer_key() -> ResidentKeyIdV1 {
    ResidentKeyIdV1::parse("key-1").expect("issuer key")
}

fn key_epoch() -> PrincipalKeyEpochV1 {
    PrincipalKeyEpochV1::new(1).expect("principal key epoch")
}

fn authority_key_epoch() -> SigningKeyEpochV1 {
    SigningKeyEpochV1::new(1).expect("delegated signing key epoch")
}

fn signer<'a>(
    principal_id: &'a ResidentPrincipalIdV1,
    key_id: &'a ResidentKeyIdV1,
) -> PrincipalKeyAdmissionContextV1<'a> {
    PrincipalKeyAdmissionContextV1::new(principal_id, key_id, key_epoch())
}

fn active_writer_head<'a>(
    writer_fence_ref: &'a ResidentRecordRefV1,
    writer_fence: &'a PrincipalResidentWriterFenceV1,
) -> PrincipalResidentWriterFenceHeadAdmissionStateV1<'a> {
    PrincipalResidentWriterFenceHeadAdmissionStateV1::Active {
        writer_fence_ref,
        writer_fence,
    }
}

fn retired_writer_head<'a>(
    writer_fence_ref: &'a ResidentRecordRefV1,
    writer_fence: &'a PrincipalResidentWriterFenceV1,
) -> PrincipalResidentWriterFenceHeadAdmissionStateV1<'a> {
    PrincipalResidentWriterFenceHeadAdmissionStateV1::Retired {
        writer_fence_ref,
        writer_fence,
    }
}

fn terminal_context<'a>(
    terminal_receipt_ref: &'a ResidentRecordRefV1,
    terminal_receipt: &'a PrincipalResidentWriterTerminalReceiptV1,
) -> PrincipalResidentWriterTerminalAdmissionContextV1<'a> {
    PrincipalResidentWriterTerminalAdmissionContextV1 {
        terminal_receipt_ref,
        terminal_receipt,
    }
}

fn takeover_context<'a>(
    takeover_receipt_ref: &'a ResidentRecordRefV1,
    takeover_receipt: &'a PrincipalResidentWriterTakeoverReceiptV1,
) -> PrincipalResidentWriterTakeoverAdmissionContextV1<'a> {
    PrincipalResidentWriterTakeoverAdmissionContextV1 {
        takeover_receipt_ref,
        takeover_receipt,
    }
}

fn writer_preimage() -> PrincipalResidentWriterFencePreimageV1 {
    PrincipalResidentWriterFencePreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA)
            .expect("writer schema"),
        schema_sha256: schema_digest('a'),
        writer_fence_id: PrincipalResidentWriterFenceIdV1::parse("writer-fence-1")
            .expect("writer fence id"),
        principal_id: principal(),
        writer_generation: PrincipalResidentWriterGenerationV1::new(1).expect("writer generation"),
        previous_writer_fence_ref: None,
        previous_writer_fence_sha256: None,
        prior_terminal_receipt_ref: None,
        prior_terminal_receipt_sha256: None,
        takeover_receipt_ref: None,
        takeover_receipt_sha256: None,
        primary_node_id: ResidentNodeIdV1::parse("node-1").expect("node"),
        process_instance_id: ResidentProcessInstanceIdV1::parse("process-1").expect("process"),
        boot_incarnation_id: ResidentBootIncarnationIdV1::parse("boot-1").expect("boot"),
        trust_store_id: ControlStoreIdV1::parse("trust-store-1").expect("trust store"),
        trust_store_incarnation_id: ControlIncarnationIdV1::parse("trust-incarnation-1")
            .expect("trust incarnation"),
        opened_deployment_store_heads_root_sha256: artifact('b'),
        principal_session_floor_epoch: PrincipalSessionFloorEpochV1::new(1).expect("session floor"),
        state: PrincipalResidentWriterFenceStateV1::Active,
        acquired_at: UnixTimeV1::new(10).expect("valid fixture time"),
        retired_at: None,
        issuer_principal_id: principal(),
        issuer_key_id: issuer_key(),
        issuer_key_epoch: key_epoch(),
    }
}

fn writer(preimage: PrincipalResidentWriterFencePreimageV1) -> PrincipalResidentWriterFenceV1 {
    PrincipalResidentWriterFenceV1::seal(preimage, signature()).expect("writer fence")
}

fn writer_wire(
    preimage: PrincipalResidentWriterFencePreimageV1,
) -> PrincipalResidentWriterFenceWireV1 {
    wire_from(&writer(preimage))
}

fn terminal_preimage(
    writer_ref: &ResidentRecordRefV1,
    writer: &PrincipalResidentWriterFenceV1,
) -> PrincipalResidentWriterTerminalReceiptPreimageV1 {
    PrincipalResidentWriterTerminalReceiptPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA)
            .expect("terminal schema"),
        schema_sha256: schema_digest('c'),
        terminal_receipt_id: PrincipalResidentWriterTerminalReceiptIdV1::parse("terminal-1")
            .expect("terminal id"),
        principal_id: writer.principal_id().clone(),
        writer_fence_ref: writer_ref.clone(),
        writer_fence_sha256: writer
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
        writer_generation: *writer.writer_generation(),
        final_trust_checkpoint_ref: record_ref("trust-checkpoint-1"),
        final_trust_checkpoint_sha256: principal_trust_checkpoint_digest('d'),
        final_opened_store_heads_root_sha256: artifact('e'),
        final_pending_custody_root_sha256: artifact('f'),
        stopped_at: UnixTimeV1::new(20).expect("valid fixture time"),
        issuer_principal_id: principal(),
        issuer_key_id: issuer_key(),
        issuer_key_epoch: key_epoch(),
    }
}

fn terminal(
    preimage: PrincipalResidentWriterTerminalReceiptPreimageV1,
) -> PrincipalResidentWriterTerminalReceiptV1 {
    PrincipalResidentWriterTerminalReceiptV1::seal(preimage, signature()).expect("terminal")
}

fn terminal_wire(
    preimage: PrincipalResidentWriterTerminalReceiptPreimageV1,
) -> PrincipalResidentWriterTerminalReceiptWireV1 {
    wire_from(&terminal(preimage))
}

fn takeover_preimage(
    active_ref: &ResidentRecordRefV1,
    active: &PrincipalResidentWriterFenceV1,
) -> PrincipalResidentWriterTakeoverReceiptPreimageV1 {
    PrincipalResidentWriterTakeoverReceiptPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA)
            .expect("takeover schema"),
        schema_sha256: schema_digest('6'),
        takeover_receipt_id: PrincipalResidentWriterTakeoverReceiptIdV1::parse("takeover-1")
            .expect("takeover id"),
        principal_id: principal(),
        predecessor_writer_fence_ref: active_ref.clone(),
        predecessor_writer_fence_sha256: active
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
        predecessor_writer_generation: *active.writer_generation(),
        predecessor_non_liveness_evidence_root_sha256: artifact('7'),
        accepted_trust_checkpoint_ref: record_ref("trust-checkpoint-2"),
        accepted_trust_checkpoint_sha256: principal_trust_checkpoint_digest('8'),
        verified_store_heads_root_sha256: artifact('9'),
        prior_session_floor_epoch: *active.principal_session_floor_epoch(),
        new_session_floor_epoch: PrincipalSessionFloorEpochV1::new(
            active.principal_session_floor_epoch().get() + 1,
        )
        .expect("new floor"),
        invalidated_route_lease_heads_root_sha256: artifact('a'),
        quarantined_custody_root_sha256: artifact('b'),
        successor_primary_node_id: ResidentNodeIdV1::parse("node-2").expect("node"),
        successor_process_instance_id: ResidentProcessInstanceIdV1::parse("process-2")
            .expect("process"),
        successor_boot_incarnation_id: ResidentBootIncarnationIdV1::parse("boot-2").expect("boot"),
        takeover_nonce: PrincipalResidentWriterTakeoverNonceV1::parse("nonce-1").expect("nonce"),
        authorized_at: UnixTimeV1::new(20).expect("valid fixture time"),
        issuer_principal_id: principal(),
        issuer_key_id: issuer_key(),
        issuer_key_epoch: key_epoch(),
    }
}

fn producer_preimage(epoch: char) -> ProducerIncarnationPreimageV1 {
    let authority = producer_authority_record(producer_authority_preimage());
    ProducerIncarnationPreimageV1 {
        schema_id: SchemaId::parse(PRODUCER_INCARNATION_V1_SCHEMA).expect("producer schema"),
        schema_sha256: schema_digest('1'),
        incarnation_id: ProducerIncarnationIdV1::parse(format!("incarnation-{epoch}"))
            .expect("incarnation id"),
        origin_principal_id: principal(),
        producer_id: ProducerIdV1::parse("producer-1").expect("producer"),
        producer_epoch: ProducerEpochV1::parse(std::iter::repeat_n(epoch, 32).collect::<String>())
            .expect("producer epoch"),
        producer_implementation_ref: record_ref("producer-implementation-1"),
        producer_implementation_sha256: artifact('2'),
        producer_incarnation_authority_ref: record_ref("producer-authority-1"),
        producer_incarnation_authority_sha256: authority
            .canonical_producer_authority_sha256()
            .clone(),
        allocated_at: UnixTimeV1::new(10).expect("valid fixture time"),
        previous_incarnation_ref: None,
        previous_incarnation_sha256: None,
        issuer_principal_id: principal(),
        issuer_key_id: issuer_key(),
        issuer_key_epoch: authority_key_epoch(),
    }
}

fn producer(preimage: ProducerIncarnationPreimageV1) -> ProducerIncarnationV1 {
    ProducerIncarnationV1::seal(preimage, signature()).expect("producer incarnation")
}

fn producer_wire(preimage: ProducerIncarnationPreimageV1) -> ProducerIncarnationWireV1 {
    wire_from(&producer(preimage))
}

fn bind_producer_authority(
    mut preimage: ProducerIncarnationPreimageV1,
    authority: &ProducerIncarnationAuthorityV1,
) -> ProducerIncarnationPreimageV1 {
    preimage.origin_principal_id = authority.principal_id().clone();
    preimage.issuer_principal_id = authority.principal_id().clone();
    preimage.issuer_key_id = authority.subject_key_id().clone();
    preimage.issuer_key_epoch = *authority.subject_key_epoch();
    preimage.producer_incarnation_authority_sha256 =
        authority.canonical_producer_authority_sha256().clone();
    preimage
}

fn producer_authority_preimage() -> ProducerIncarnationAuthorityPreimageV1 {
    ProducerIncarnationAuthorityPreimageV1 {
        schema_id: SchemaId::parse(PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA)
            .expect("producer authority schema"),
        schema_sha256: schema_digest('9'),
        producer_authority_id: ProducerIncarnationAuthorityIdV1::parse("producer-authority-1")
            .expect("producer authority id"),
        principal_id: principal(),
        subject_key_id: issuer_key(),
        subject_key_epoch: authority_key_epoch(),
        subject_public_key: Ed25519PublicKeyV1::parse("M".repeat(43))
            .expect("producer authority public key"),
        permitted_producer_namespaces: vec![
            ProducerNamespaceV1::parse("pale").expect("producer namespace")
        ],
        authority_epoch: ProducerIncarnationAuthorityEpochV1::new(1)
            .expect("producer authority epoch"),
        previous_authority_ref: None,
        previous_authority_sha256: None,
        predecessor_final_publication_frontier_ref: None,
        predecessor_final_publication_frontier_sha256: None,
        state: PrincipalAuthorityStateV1::Active,
        issued_at: UnixTimeV1::new(5).expect("producer authority issue time"),
        expires_at: UnixTimeV1::new(20).expect("producer authority expiry time"),
        issuer_principal_id: principal(),
        issuer_key_id: issuer_key(),
        issuer_key_epoch: key_epoch(),
    }
}

fn producer_authority_record(
    preimage: ProducerIncarnationAuthorityPreimageV1,
) -> ProducerIncarnationAuthorityV1 {
    ProducerIncarnationAuthorityV1::seal(preimage, signature()).expect("producer authority")
}

fn producer_authority_context<'a>(
    authority_ref: &'a ResidentRecordRefV1,
    authority: &'a ProducerIncarnationAuthorityV1,
) -> ProducerIncarnationAuthorityAdmissionContextV1<'a> {
    ProducerIncarnationAuthorityAdmissionContextV1::new(authority_ref, authority)
        .expect("self-consistent producer authority")
}

fn fixture_namespace_coverage(
    namespaces: &[ProducerNamespaceV1],
    producer_id: &ProducerIdV1,
) -> bool {
    namespaces.len() == 1
        && namespaces[0].as_str() == "pale"
        && producer_id.as_str() == "producer-1"
}

fn wire_from<T, W>(value: &T) -> W
where
    T: serde::Serialize,
    W: serde::de::DeserializeOwned,
{
    serde_json::from_value(serde_json::to_value(value).expect("record serializes"))
        .expect("structural wire decodes")
}
