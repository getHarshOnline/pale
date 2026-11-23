//! Admitted principal authority inputs for interface conformance.
//!
//! The checked fixture contains signed bootstrap candidates only. Every wire
//! passes the ordinary record-specific and contextual admission functions
//! before use. Its deterministic signatures are test evidence; they grant
//! no production authority and prove no key custody.

use pale_core::records::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const SIGNATURE: &str =
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureRecord<W> {
    #[serde(rename = "recordRef")]
    record_ref: ResidentRecordRefV1,
    wire: W,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BootstrapAuthorityInputs {
    principal: FixtureRecord<PrincipalRecordWireV1>,
    recovery_policy: FixtureRecord<PrincipalRecoveryPolicyWireV1>,
    local_admin_authority: FixtureRecord<LocalAdminAuthorityWireV1>,
    producer_authority: FixtureRecord<ProducerIncarnationAuthorityWireV1>,
    checkpoint_authority: FixtureRecord<CheckpointSigningAuthorityWireV1>,
    trust_checkpoint: FixtureRecord<PrincipalTrustCheckpointWireV1>,
    control_checkpoints: Vec<FixtureRecord<ControlStateCheckpointWireV1>>,
    bootstrap_receipt: FixtureRecord<PrincipalBootstrapReceiptWireV1>,
}

pub(in super::super) fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

pub(in super::super) fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse(SIGNATURE).expect("deterministic signature")
}

fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
        .expect("fixture schema digest")
}

fn root_public_key() -> Ed25519PublicKeyV1 {
    Ed25519PublicKeyV1::parse("A".repeat(43)).expect("fixture root public key")
}

fn checkpoint_public_key() -> Ed25519PublicKeyV1 {
    Ed25519PublicKeyV1::parse("Q".repeat(43)).expect("fixture checkpoint public key")
}

pub(in super::super) fn signed_wire<P, D, W>(preimage: &P, digest_field: &str, digest: &D) -> W
where
    P: Serialize,
    D: Serialize,
    W: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    let object = value.as_object_mut().expect("preimage is an object");
    object.insert(
        digest_field.into(),
        serde_json::to_value(digest).expect("digest serializes"),
    );
    object.insert(
        "signature".into(),
        serde_json::to_value(signature()).expect("signature serializes"),
    );
    serde_json::from_value(value).expect("signed wire decodes")
}

pub(super) fn fixture_inputs_from_str(
    source: &str,
) -> Result<BootstrapAuthorityInputs, serde_json::Error> {
    serde_json::from_str(source)
}

fn fixture_inputs() -> BootstrapAuthorityInputs {
    fixture_inputs_from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resident/invoke-conformance.v1/principal/bootstrap-authority-inputs.json"
    )))
    .expect("bootstrap authority fixture is valid JSON")
}

/// Admit the complete checked bootstrap fixture through its owning records.
pub(in super::super) fn admitted_bootstrap() -> AdmittedPrincipalBootstrapV1 {
    admit_bootstrap(fixture_inputs()).expect("principal bootstrap fixture admits")
}

pub(super) fn admit_bootstrap(
    inputs: BootstrapAuthorityInputs,
) -> Result<AdmittedPrincipalBootstrapV1, InvalidResidentRecordV1> {
    let root_public_key = root_public_key();
    let principal_schema = schema_digest('2');
    let recovery_schema = schema_digest('1');
    let genesis = AdmittedPrincipalGenesisV1::admit_initial(
        InitialPrincipalGenesisCandidateV1 {
            principal_record_ref: inputs.principal.record_ref,
            principal_wire: inputs.principal.wire,
            expected_principal_schema_sha256: &principal_schema,
            recovery_policy_ref: inputs.recovery_policy.record_ref,
            recovery_wire: inputs.recovery_policy.wire,
            expected_recovery_schema_sha256: &recovery_schema,
        },
        |_, key, _, _, _| {
            (key == &root_public_key).then_some(()).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "interface conformance principal",
                    invariant: "deterministic root signer",
                },
            )
        },
        |_, key, _, _, _| {
            (key == &root_public_key).then_some(()).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "interface conformance recovery policy",
                    invariant: "deterministic root signer",
                },
            )
        },
    )?;

    let admin_schema = schema_digest('3');
    let producer_schema = schema_digest('4');
    let checkpoint_authority_schema = schema_digest('5');
    let checkpoint_public_key = checkpoint_public_key();
    let bootstrap_id = PrincipalBootstrapIdV1::parse("interface-conformance-bootstrap")
        .expect("fixture bootstrap identity");
    let authorities = genesis.admit_initial_authorities(
        InitialPrincipalAuthoritiesCandidateV1 {
            local_admin_authority_ref: inputs.local_admin_authority.record_ref,
            local_admin_wire: inputs.local_admin_authority.wire,
            expected_local_admin_schema_sha256: &admin_schema,
            producer_authority_ref: inputs.producer_authority.record_ref,
            producer_wire: inputs.producer_authority.wire,
            expected_producer_schema_sha256: &producer_schema,
            checkpoint_authority_ref: inputs.checkpoint_authority.record_ref,
            checkpoint_wire: inputs.checkpoint_authority.wire,
            expected_checkpoint_schema_sha256: &checkpoint_authority_schema,
            expected_bootstrap_id: &bootstrap_id,
        },
        |_, key, _, _, _| {
            (key == &root_public_key).then_some(()).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "interface conformance admin authority",
                    invariant: "deterministic root signer",
                },
            )
        },
        |_, key, _, _, _| {
            (key == &root_public_key).then_some(()).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "interface conformance producer authority",
                    invariant: "deterministic root signer",
                },
            )
        },
        |_, key, _, _, _| {
            (key == &root_public_key).then_some(()).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "interface conformance checkpoint authority",
                    invariant: "deterministic root signer",
                },
            )
        },
    )?;

    let trust_schema = schema_digest('7');
    let trust_store_id = ControlStoreIdV1::parse("trust-store-1").expect("fixture trust store");
    let trust_store_incarnation_id = ControlIncarnationIdV1::parse("trust-incarnation-1")
        .expect("fixture trust-store incarnation");
    let principal_control_store_id = ControlStoreIdV1::parse("principal-control-store-1")
        .expect("fixture principal control store");
    let principal_control_incarnation_id =
        ControlIncarnationIdV1::parse("principal-control-incarnation-1")
            .expect("fixture principal control-store incarnation");
    let checkpoint_profile_ref = record_ref("checkpoint-profile-1");
    let checkpoint_profile_sha256 =
        CheckpointProfileDigest::parse("6".repeat(64)).expect("fixture checkpoint profile digest");
    let required_domains = ["admin-authority", "principal-head"]
        .into_iter()
        .map(|domain| {
            ControlCheckpointDomainIdV1::parse(domain).expect("fixture checkpoint domain")
        })
        .collect::<Vec<_>>();
    let control_checkpoint_schema = schema_digest('8');
    let profile = PrincipalBootstrapCheckpointProfileV1 {
        trust_store_id: &trust_store_id,
        trust_store_incarnation_id: &trust_store_incarnation_id,
        principal_control_store_id: &principal_control_store_id,
        principal_control_incarnation_id: &principal_control_incarnation_id,
        checkpoint_profile_ref: &checkpoint_profile_ref,
        checkpoint_profile_sha256: &checkpoint_profile_sha256,
        required_checkpoint_domains: &required_domains,
        control_checkpoint_schema_sha256: &control_checkpoint_schema,
        control_schema_version: 1,
    };
    let checkpoint_candidates = inputs
        .control_checkpoints
        .into_iter()
        .map(|entry| (entry.record_ref, entry.wire))
        .collect();
    let frontier = authorities.admit_initial_trust_frontier(
        InitialPrincipalTrustFrontierCandidateV1 {
            trust_checkpoint_ref: inputs.trust_checkpoint.record_ref,
            trust_wire: inputs.trust_checkpoint.wire,
            expected_trust_schema_sha256: &trust_schema,
            profile,
            checkpoint_candidates,
        },
        |_, key, _, _, _| {
            (key == &root_public_key).then_some(()).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "interface conformance trust checkpoint",
                    invariant: "deterministic root signer",
                },
            )
        },
        |_, key, _, _, _| {
            (key == &checkpoint_public_key).then_some(()).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "interface conformance control checkpoint",
                    invariant: "deterministic checkpoint signer",
                },
            )
        },
        |prefixes, domain| {
            prefixes
                .iter()
                .any(|prefix| prefix.as_str() == domain.as_str())
        },
    )?;

    let receipt_schema = schema_digest('d');
    frontier.admit_receipt(
        inputs.bootstrap_receipt.record_ref,
        inputs.bootstrap_receipt.wire,
        &receipt_schema,
        |_, key, _, _, _| {
            (key == &root_public_key).then_some(()).ok_or(
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "interface conformance bootstrap receipt",
                    invariant: "deterministic root signer",
                },
            )
        },
    )
}
