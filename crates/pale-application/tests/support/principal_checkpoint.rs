//! Shared initial checkpoint construction using the supplied custody port.
//!
//! Signature callbacks check the existing fixture identities; they are not
//! cryptographic qualification. Native tests supply the real tree producer.

use pale_core::records::*;
use pale_ports::{
    AccumulatorPreparationLimits, AuthorityStoreCoordinate, PrincipalBootstrapCustodyPort,
};

use super::{principal_trust_test_support as bootstrap, resident};

pub(crate) struct BootstrapRecords {
    pub(crate) trust_ref: ResidentRecordRefV1,
    pub(crate) trust: PrincipalTrustCheckpointV1,
    pub(crate) signer_ref: ResidentRecordRefV1,
    pub(crate) signer: CheckpointSigningAuthorityV1,
}

impl BootstrapRecords {
    pub(crate) fn authority<'a>(
        &'a self,
        control: &'a ControlStateCheckpointPreimageV1,
    ) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'a> {
        ControlCheckpointSigningAuthorityAdmissionContextV1 {
            authority_ref: &self.signer_ref,
            authority_sha256: self.signer.canonical_checkpoint_signing_authority_sha256(),
            principal_id: self.signer.principal_id(),
            control_store_id: self.signer.control_store_id(),
            control_incarnation_id: self.signer.control_incarnation_id(),
            checkpoint_domain_id: &control.checkpoint_domain_id,
            checkpoint_profile_ref: self.signer.checkpoint_profile_ref(),
            checkpoint_profile_sha256: self.signer.checkpoint_profile_sha256(),
            subject_key_id: self.signer.subject_key_id(),
            subject_key_epoch: *self.signer.subject_key_epoch(),
        }
    }

    pub(crate) fn checkpoint(
        &self,
        control: &ControlStateCheckpointPreimageV1,
    ) -> ControlStateCheckpointV1 {
        ControlStateCheckpointV1::issue_initial_contextual(
            control.clone(),
            &control.schema_sha256,
            &self.authority(control),
            |_, _, _| Ok(CanonicalSignatureV1::parse("A".repeat(86)).unwrap()),
        )
        .unwrap()
    }
}

/// Run the actual application preparation and admissions without committing.
/// The callback can alter the signed candidate for refusal tests; it cannot
/// replace the preparation or bypass its admission checks.
pub(crate) fn prepare(
    custody: &mut dyn PrincipalBootstrapCustodyPort,
    adjust: impl FnOnce(&mut ControlStateCheckpointPreimageV1),
) -> Result<
    (
        resident::PreparedPrincipalBootstrapCustody,
        ControlStateCheckpointPreimageV1,
    ),
    resident::PrincipalCheckpointError,
> {
    prepare_with_records(custody, adjust).map(|(custody, control, _)| (custody, control))
}

/// Retain the same admitted records when a test continues beyond bootstrap.
pub(crate) fn prepare_with_records(
    custody: &mut dyn PrincipalBootstrapCustodyPort,
    adjust: impl FnOnce(&mut ControlStateCheckpointPreimageV1),
) -> Result<
    (
        resident::PreparedPrincipalBootstrapCustody,
        ControlStateCheckpointPreimageV1,
        BootstrapRecords,
    ),
    resident::PrincipalCheckpointError,
> {
    let profile = resident::SupportedPrincipalTrustCheckpointProfile::admit(
        reference("source-principal-profile"),
        &resident::SupportedPrincipalTrustCheckpointProfile::content_digest()?,
        resident::SupportedPrincipalTrustCheckpointProfile::canonical_content()?.as_bytes(),
    )?;
    let trust_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("trust-store-1")?,
        ControlIncarnationIdV1::parse("trust-incarnation-1")?,
    );
    let control_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("principal-control-store-1")?,
        ControlIncarnationIdV1::parse("principal-control-incarnation-1")?,
    );
    let schema = SchemaDigest::parse("8".repeat(64)).expect("fixture schema digest");
    let selection = profile.bootstrap_profile(&trust_store, &control_store, &schema);
    let bootstrap_id = PrincipalBootstrapIdV1::parse("source-principal-bootstrap")?;
    let authorities = bootstrap::admit_profile_authorities(&bootstrap_id, &selection)?;
    let trust_preimage = bootstrap::initial_trust_preimage(&authorities, Some(&selection));
    let trust_digest = PrincipalTrustCheckpointDigest::hash_canonical(&trust_preimage)?;
    let trust_ref = reference("source-principal-trust-head");
    let root_key = authorities
        .genesis()
        .principal_record()
        .root_public_key()
        .clone();
    let signer = authorities.checkpoint_authority();
    let signer_digest = signer
        .canonical_checkpoint_signing_authority_sha256()
        .clone();
    let signer_key = signer.subject_public_key().clone();
    let signer_ref = authorities.checkpoint_authority_ref().clone();
    let retained_signer = signer.clone();
    let mut control = ControlStateCheckpointPreimageV1 {
        schema_id: SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA).expect("checkpoint schema"),
        schema_sha256: schema.clone(),
        control_checkpoint_id: ControlCheckpointIdV1::parse("source-principal-control-head")?,
        control_store_id: control_store.store_id().clone(),
        control_incarnation_id: control_store.incarnation_id().clone(),
        checkpoint_domain_id: selection.required_checkpoint_domains[0].clone(),
        domain_epoch: ControlDomainEpochV1::new(1)?,
        domain_sequence: ControlDomainSequenceV1::new(1)?,
        previous_control_checkpoint_ref: None,
        previous_control_checkpoint_sha256: None,
        checkpoint_profile_ref: selection.checkpoint_profile_ref.clone(),
        checkpoint_profile_sha256: selection.checkpoint_profile_sha256.clone(),
        control_schema_version: selection.control_schema_version,
        committed_transaction_sequence: ControlTransactionSequenceV1::new(2)?,
        state_root_sha256: ArtifactDigest::parse("0".repeat(64)).expect("empty root"),
        leaf_count: ControlLeafCountV1::new(2)?,
        changed_leaf_keys: Vec::new(),
        unresolved_custody_root_sha256: ArtifactDigest::parse("0".repeat(64)).expect("empty root"),
        issued_at: UnixTimeV1::new(45)?,
        checkpoint_signing_authority_ref: authorities.checkpoint_authority_ref().clone(),
        checkpoint_signing_authority_sha256: signer_digest.clone(),
        issuer_principal_id: signer.principal_id().clone(),
        issuer_key_id: signer.subject_key_id().clone(),
        issuer_key_epoch: *signer.subject_key_epoch(),
    };
    let admitted_trust = authorities.admit_initial_trust_checkpoint(
        trust_ref.clone(),
        bootstrap::signed_wire(
            &trust_preimage,
            "canonical_trust_checkpoint_sha256",
            &trust_digest,
        ),
        &trust_preimage.schema_sha256,
        selection,
        |domain, key, _, _, _| {
            assert_eq!(domain, PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &root_key);
            Ok(())
        },
    )?;
    let records = BootstrapRecords {
        trust_ref: trust_ref.clone(),
        trust: admitted_trust.trust_checkpoint_entry().1.clone(),
        signer_ref,
        signer: retained_signer,
    };
    let prepared = profile.prepare_bootstrap(
        admitted_trust,
        AccumulatorPreparationLimits::new(2, 4096, 4096, 4096, 262_144)?,
        custody,
    )?;
    control.committed_transaction_sequence = prepared.transaction_sequence();
    control.state_root_sha256 = prepared.state_root().clone();
    control.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    control.changed_leaf_keys = prepared.changed_keys().to_vec();
    adjust(&mut control);
    let control_digest = ControlStateCheckpointDigest::hash_canonical(&control)?;
    let control_ref = reference("source-principal-control-head");
    let frontier = prepared.admit_control_checkpoint(
        control_ref.clone(),
        bootstrap::signed_wire(
            &control,
            "canonical_control_checkpoint_sha256",
            &control_digest,
        ),
        |domain, key, _, _, _| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &signer_key);
            Ok(())
        },
        |prefixes, domain| {
            prefixes
                .iter()
                .any(|prefix| prefix.as_str() == domain.as_str())
        },
    )?;
    let receipt = bootstrap::bootstrap_receipt_preimage(
        bootstrap_id,
        trust_preimage,
        signer_digest,
        trust_ref,
        trust_digest,
        vec![PrincipalBootstrapInitialControlCheckpointV1 {
            checkpoint_domain_id: control.checkpoint_domain_id.clone(),
            domain_epoch: control.domain_epoch,
            domain_sequence: control.domain_sequence,
            control_checkpoint_ref: control_ref,
            control_checkpoint_sha256: control_digest,
        }],
        50,
    );
    let receipt_digest = PrincipalBootstrapReceiptDigest::hash_canonical(&receipt)?;
    let prepared = frontier.admit_receipt(
        reference("source-principal-bootstrap-receipt"),
        bootstrap::signed_wire(
            &receipt,
            "canonical_bootstrap_receipt_sha256",
            &receipt_digest,
        ),
        &receipt.schema_sha256,
        |domain, key, _, _, _| {
            assert_eq!(domain, PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &root_key);
            Ok(())
        },
    )?;
    Ok((prepared, control, records))
}

fn reference(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("fixture record reference")
}

/// Build the signed candidate from the production preparation, not expected roots.
pub(crate) fn finalization_preimage(
    prepared: &resident::PreparedDeploymentFinalizationCheckpoint<'_>,
    prior_ref: &ResidentRecordRefV1,
    prior: &ControlStateCheckpointV1,
    mut preimage: ControlStateCheckpointPreimageV1,
) -> ControlStateCheckpointPreimageV1 {
    preimage.control_checkpoint_id =
        ControlCheckpointIdV1::parse("finalization-control-head").unwrap();
    preimage.domain_sequence =
        ControlDomainSequenceV1::new(prior.domain_sequence().get() + 1).unwrap();
    preimage.previous_control_checkpoint_ref = Some(prior_ref.clone());
    preimage.previous_control_checkpoint_sha256 =
        Some(prior.canonical_control_checkpoint_sha256().clone());
    preimage.issued_at = UnixTimeV1::new(65).unwrap();
    preimage.state_root_sha256 = prepared.state_root().clone();
    preimage.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    preimage.changed_leaf_keys = prepared.changed_keys().to_vec();
    preimage.leaf_count = prepared.leaf_count();
    preimage.committed_transaction_sequence = prepared.transaction_sequence();
    preimage
}
