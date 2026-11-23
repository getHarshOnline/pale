//! Application projection laws for clean principal resident-writer transitions.

use serde::{de::DeserializeOwned, Serialize};

use super::{
    prepare_clean_principal_resident_writer_retirement,
    prepare_clean_principal_resident_writer_successor,
    prepare_initial_principal_resident_writer_transition,
    CleanPrincipalResidentWriterTransitionPreimage, PrincipalResidentWriterProjectionError,
};
use pale_core::records::*;
use pale_ports::{
    AuthorityStoreCoordinate, PrincipalResidentWriterTransitionKind, PrincipalTrustStoreBinding,
};

/// Owner-resolved identities shared by the admitted fixture records.
struct FixtureAuthority {
    principal_id: ResidentPrincipalIdV1,
    principal_key_id: ResidentKeyIdV1,
    principal_key_epoch: PrincipalKeyEpochV1,
}

impl FixtureAuthority {
    /// Build the exact principal-key witness used by writer admission.
    fn principal_key(&self) -> PrincipalKeyAdmissionContextV1<'_> {
        PrincipalKeyAdmissionContextV1::new(
            &self.principal_id,
            &self.principal_key_id,
            self.principal_key_epoch,
        )
    }
}

/// Complete admitted semantic fixture for the three clean projections.
struct Fixture {
    physical_trust_store: PrincipalTrustStoreBinding,
    principal_control_store: AuthorityStoreCoordinate,
    active_ref: ResidentRecordRefV1,
    active: PrincipalResidentWriterFenceV1,
    terminal_ref: ResidentRecordRefV1,
    terminal: PrincipalResidentWriterTerminalReceiptV1,
    retired_ref: ResidentRecordRefV1,
    retired: PrincipalResidentWriterFenceV1,
    successor_ref: ResidentRecordRefV1,
    successor: PrincipalResidentWriterFenceV1,
    checkpoint_refs: [ResidentRecordRefV1; 4],
    checkpoints: [ControlStateCheckpointV1; 4],
}

impl Fixture {
    /// Bind the active-to-retired records and their checkpoint transition.
    fn retirement_preimage(&self) -> CleanPrincipalResidentWriterTransitionPreimage<'_> {
        CleanPrincipalResidentWriterTransitionPreimage::new(
            (&self.active_ref, &self.active),
            (&self.terminal_ref, &self.terminal),
            (&self.retired_ref, &self.retired),
            (&self.checkpoint_refs[1], &self.checkpoints[1]),
            (&self.checkpoint_refs[2], &self.checkpoints[2]),
        )
    }

    /// Bind the retired-to-successor records and their checkpoint transition.
    fn successor_preimage(&self) -> CleanPrincipalResidentWriterTransitionPreimage<'_> {
        CleanPrincipalResidentWriterTransitionPreimage::new(
            (&self.retired_ref, &self.retired),
            (&self.terminal_ref, &self.terminal),
            (&self.successor_ref, &self.successor),
            (&self.checkpoint_refs[2], &self.checkpoints[2]),
            (&self.checkpoint_refs[3], &self.checkpoints[3]),
        )
    }
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

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

fn authority(principal: &str) -> FixtureAuthority {
    FixtureAuthority {
        principal_id: ResidentPrincipalIdV1::parse(principal).expect("principal"),
        principal_key_id: ResidentKeyIdV1::parse("key-1").expect("principal key"),
        principal_key_epoch: PrincipalKeyEpochV1::new(1).expect("principal key epoch"),
    }
}

/// Construct an untrusted wire through its public deserialization boundary.
fn signed_wire<P, D, W>(preimage: &P, digest: &D, digest_field: &str) -> W
where
    P: Serialize,
    D: Serialize,
    W: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    let object = value.as_object_mut().expect("preimage is an object");
    object.insert(
        digest_field.to_owned(),
        serde_json::to_value(digest).expect("digest serializes"),
    );
    object.insert(
        "signature".to_owned(),
        serde_json::to_value(signature()).expect("signature serializes"),
    );
    serde_json::from_value(value).expect("wire deserializes")
}

fn writer_preimage(authority: &FixtureAuthority) -> PrincipalResidentWriterFencePreimageV1 {
    PrincipalResidentWriterFencePreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA)
            .expect("writer schema"),
        schema_sha256: schema_digest('a'),
        writer_fence_id: PrincipalResidentWriterFenceIdV1::parse("writer-fence-1")
            .expect("writer fence id"),
        principal_id: authority.principal_id.clone(),
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
        acquired_at: UnixTimeV1::new(10).expect("fixture time"),
        retired_at: None,
        issuer_principal_id: authority.principal_id.clone(),
        issuer_key_id: authority.principal_key_id.clone(),
        issuer_key_epoch: authority.principal_key_epoch,
    }
}

fn admit_initial_writer(
    authority: &FixtureAuthority,
    preimage: PrincipalResidentWriterFencePreimageV1,
) -> PrincipalResidentWriterFenceV1 {
    let digest =
        PrincipalResidentWriterFenceDigest::hash_canonical(&preimage).expect("writer digest");
    let wire = signed_wire(
        &preimage,
        &digest,
        "canonical_principal_resident_writer_fence_sha256",
    );
    PrincipalResidentWriterFenceV1::admit_initial_active_wire(
        wire,
        &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
        &preimage.schema_sha256,
        &authority.principal_key(),
        |domain, _, _, _| {
            assert_eq!(domain, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN);
            Ok(())
        },
    )
    .expect("initial writer admits")
}

fn terminal_preimage(
    authority: &FixtureAuthority,
    active_ref: &ResidentRecordRefV1,
    active: &PrincipalResidentWriterFenceV1,
    receipt_id: &str,
) -> PrincipalResidentWriterTerminalReceiptPreimageV1 {
    PrincipalResidentWriterTerminalReceiptPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA)
            .expect("terminal schema"),
        schema_sha256: schema_digest('c'),
        terminal_receipt_id: PrincipalResidentWriterTerminalReceiptIdV1::parse(receipt_id)
            .expect("terminal receipt id"),
        principal_id: authority.principal_id.clone(),
        writer_fence_ref: active_ref.clone(),
        writer_fence_sha256: active
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
        writer_generation: *active.writer_generation(),
        final_trust_checkpoint_ref: record_ref("trust-checkpoint-1"),
        final_trust_checkpoint_sha256: PrincipalTrustCheckpointDigest::parse(sha('d'))
            .expect("trust checkpoint digest"),
        final_opened_store_heads_root_sha256: artifact('e'),
        final_pending_custody_root_sha256: artifact('f'),
        stopped_at: UnixTimeV1::new(20).expect("fixture time"),
        issuer_principal_id: authority.principal_id.clone(),
        issuer_key_id: authority.principal_key_id.clone(),
        issuer_key_epoch: authority.principal_key_epoch,
    }
}

fn admit_terminal(
    authority: &FixtureAuthority,
    active_ref: &ResidentRecordRefV1,
    active: &PrincipalResidentWriterFenceV1,
    preimage: PrincipalResidentWriterTerminalReceiptPreimageV1,
) -> PrincipalResidentWriterTerminalReceiptV1 {
    let digest = PrincipalResidentWriterTerminalReceiptDigest::hash_canonical(&preimage)
        .expect("terminal digest");
    let wire = signed_wire(
        &preimage,
        &digest,
        "canonical_principal_resident_writer_terminal_receipt_sha256",
    );
    PrincipalResidentWriterTerminalReceiptV1::admit_contextual_wire(
        wire,
        &PrincipalResidentWriterFenceHeadAdmissionStateV1::active(active_ref, active)
            .expect("active writer head"),
        &preimage.schema_sha256,
        &authority.principal_key(),
        |domain, _, _, _| {
            assert_eq!(
                domain,
                PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SIGNATURE_DOMAIN
            );
            Ok(())
        },
    )
    .expect("terminal receipt admits")
}

fn retired_preimage(
    authority: &FixtureAuthority,
    active_ref: &ResidentRecordRefV1,
    active: &PrincipalResidentWriterFenceV1,
    terminal_ref: &ResidentRecordRefV1,
    terminal: &PrincipalResidentWriterTerminalReceiptV1,
) -> PrincipalResidentWriterFencePreimageV1 {
    let mut preimage = writer_preimage(authority);
    preimage.previous_writer_fence_ref = Some(active_ref.clone());
    preimage.previous_writer_fence_sha256 = Some(
        active
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
    );
    preimage.prior_terminal_receipt_ref = Some(terminal_ref.clone());
    preimage.prior_terminal_receipt_sha256 = Some(
        terminal
            .canonical_principal_resident_writer_terminal_receipt_sha256()
            .clone(),
    );
    preimage.state = PrincipalResidentWriterFenceStateV1::Retired;
    preimage.retired_at = Some(*terminal.stopped_at());
    preimage
}

fn admit_retired(
    authority: &FixtureAuthority,
    active_ref: &ResidentRecordRefV1,
    active: &PrincipalResidentWriterFenceV1,
    terminal_ref: &ResidentRecordRefV1,
    terminal: &PrincipalResidentWriterTerminalReceiptV1,
    preimage: PrincipalResidentWriterFencePreimageV1,
) -> PrincipalResidentWriterFenceV1 {
    let digest = PrincipalResidentWriterFenceDigest::hash_canonical(&preimage)
        .expect("retired writer digest");
    let wire = signed_wire(
        &preimage,
        &digest,
        "canonical_principal_resident_writer_fence_sha256",
    );
    PrincipalResidentWriterFenceV1::admit_clean_retired_wire(
        wire,
        &PrincipalResidentWriterFenceHeadAdmissionStateV1::active(active_ref, active)
            .expect("active writer head"),
        PrincipalResidentWriterTerminalAdmissionContextV1 {
            terminal_receipt_ref: terminal_ref,
            terminal_receipt: terminal,
        },
        &preimage.schema_sha256,
        &authority.principal_key(),
        |_, _, _, _| Ok(()),
    )
    .expect("retired writer admits")
}

fn successor_preimage(
    authority: &FixtureAuthority,
    retired_ref: &ResidentRecordRefV1,
    retired: &PrincipalResidentWriterFenceV1,
    terminal_ref: &ResidentRecordRefV1,
    terminal: &PrincipalResidentWriterTerminalReceiptV1,
) -> PrincipalResidentWriterFencePreimageV1 {
    let mut preimage = writer_preimage(authority);
    preimage.writer_generation =
        PrincipalResidentWriterGenerationV1::new(2).expect("successor generation");
    preimage.previous_writer_fence_ref = Some(retired_ref.clone());
    preimage.previous_writer_fence_sha256 = Some(
        retired
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
    );
    preimage.prior_terminal_receipt_ref = Some(terminal_ref.clone());
    preimage.prior_terminal_receipt_sha256 = Some(
        terminal
            .canonical_principal_resident_writer_terminal_receipt_sha256()
            .clone(),
    );
    preimage.opened_deployment_store_heads_root_sha256 =
        terminal.final_opened_store_heads_root_sha256().clone();
    preimage.process_instance_id =
        ResidentProcessInstanceIdV1::parse("process-2").expect("successor process");
    preimage.acquired_at = UnixTimeV1::new(21).expect("fixture time");
    preimage
}

fn admit_successor(
    authority: &FixtureAuthority,
    retired_ref: &ResidentRecordRefV1,
    retired: &PrincipalResidentWriterFenceV1,
    terminal_ref: &ResidentRecordRefV1,
    terminal: &PrincipalResidentWriterTerminalReceiptV1,
    preimage: PrincipalResidentWriterFencePreimageV1,
) -> PrincipalResidentWriterFenceV1 {
    let digest = PrincipalResidentWriterFenceDigest::hash_canonical(&preimage)
        .expect("successor writer digest");
    let wire = signed_wire(
        &preimage,
        &digest,
        "canonical_principal_resident_writer_fence_sha256",
    );
    PrincipalResidentWriterFenceV1::admit_clean_successor_active_wire(
        wire,
        &PrincipalResidentWriterFenceHeadAdmissionStateV1::retired(retired_ref, retired)
            .expect("retired writer head"),
        PrincipalResidentWriterTerminalAdmissionContextV1 {
            terminal_receipt_ref: terminal_ref,
            terminal_receipt: terminal,
        },
        &preimage.schema_sha256,
        &authority.principal_key(),
        |_, _, _, _| Ok(()),
    )
    .expect("successor writer admits")
}

fn checkpoint_preimage(
    sequence: u64,
    store: &str,
    incarnation: &str,
    authority: &FixtureAuthority,
) -> ControlStateCheckpointPreimageV1 {
    ControlStateCheckpointPreimageV1 {
        schema_id: SchemaId::parse("pale/control-state-checkpoint.v1").expect("checkpoint schema"),
        schema_sha256: schema_digest('1'),
        control_checkpoint_id: ControlCheckpointIdV1::parse(format!("checkpoint-{sequence}"))
            .expect("checkpoint id"),
        control_store_id: ControlStoreIdV1::parse(store).expect("control store"),
        control_incarnation_id: ControlIncarnationIdV1::parse(incarnation)
            .expect("control incarnation"),
        checkpoint_domain_id: ControlCheckpointDomainIdV1::parse("principal-resident-writer")
            .expect("checkpoint domain"),
        domain_epoch: ControlDomainEpochV1::new(1).expect("domain epoch"),
        domain_sequence: ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        previous_control_checkpoint_ref: None,
        previous_control_checkpoint_sha256: None,
        checkpoint_profile_ref: record_ref("checkpoint-profile"),
        checkpoint_profile_sha256: CheckpointProfileDigest::parse(sha('2'))
            .expect("checkpoint profile digest"),
        control_schema_version: 1,
        committed_transaction_sequence: ControlTransactionSequenceV1::new(20 + sequence)
            .expect("transaction sequence"),
        state_root_sha256: artifact('3'),
        leaf_count: ControlLeafCountV1::new(1).expect("leaf count"),
        changed_leaf_keys: vec![ChangedControlLeafKeyV1 {
            record_class: ControlRecordClassV1::parse("principal-writer").expect("record class"),
            scope_key_sha256: artifact('4'),
            logical_key: ControlLogicalKeyV1::parse("writer-head").expect("logical key"),
        }],
        unresolved_custody_root_sha256: artifact('5'),
        issued_at: UnixTimeV1::new(30 + sequence).expect("fixture time"),
        checkpoint_signing_authority_ref: record_ref("checkpoint-authority"),
        checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest::parse(sha('6'))
            .expect("checkpoint authority digest"),
        issuer_principal_id: authority.principal_id.clone(),
        issuer_key_id: authority.principal_key_id.clone(),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("signing key epoch"),
    }
}

fn checkpoint_signer(
    preimage: &ControlStateCheckpointPreimageV1,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
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
    }
}

fn checkpoint_wire(preimage: &ControlStateCheckpointPreimageV1) -> ControlStateCheckpointWireV1 {
    let digest = ControlStateCheckpointDigest::hash_canonical(preimage).expect("checkpoint digest");
    signed_wire(preimage, &digest, "canonical_control_checkpoint_sha256")
}

fn admit_initial_checkpoint(
    preimage: ControlStateCheckpointPreimageV1,
) -> ControlStateCheckpointV1 {
    ControlStateCheckpointV1::admit_initial_wire(
        checkpoint_wire(&preimage),
        &preimage.schema_sha256,
        &checkpoint_signer(&preimage),
        |_, _, _, _| Ok(()),
    )
    .expect("initial checkpoint admits")
}

fn admit_checkpoint_successor(
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ControlStateCheckpointV1,
    mut preimage: ControlStateCheckpointPreimageV1,
) -> ControlStateCheckpointV1 {
    preimage.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
    preimage.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    ControlStateCheckpointV1::admit_successor_wire(
        checkpoint_wire(&preimage),
        predecessor_ref,
        predecessor,
        &preimage.schema_sha256,
        &checkpoint_signer(&preimage),
        |_, _, _, _| Ok(()),
    )
    .expect("successor checkpoint admits")
}

fn checkpoint_chain(
    authority: &FixtureAuthority,
    store: &str,
    incarnation: &str,
    prefix: &str,
    length: usize,
) -> (Vec<ResidentRecordRefV1>, Vec<ControlStateCheckpointV1>) {
    let refs = (1..=length)
        .map(|sequence| record_ref(&format!("{prefix}-{sequence}")))
        .collect::<Vec<_>>();
    let mut checkpoints = vec![admit_initial_checkpoint(checkpoint_preimage(
        1,
        store,
        incarnation,
        authority,
    ))];
    for sequence in 2..=length {
        let predecessor = checkpoints.last().expect("checkpoint predecessor");
        checkpoints.push(admit_checkpoint_successor(
            &refs[sequence - 2],
            predecessor,
            checkpoint_preimage(sequence as u64, store, incarnation, authority),
        ));
    }
    (refs, checkpoints)
}

fn fixture() -> Fixture {
    let authority = authority("principal-1");
    let active_ref = record_ref("writer-active-1");
    let active = admit_initial_writer(&authority, writer_preimage(&authority));
    let terminal_ref = record_ref("writer-terminal-1");
    let terminal = admit_terminal(
        &authority,
        &active_ref,
        &active,
        terminal_preimage(&authority, &active_ref, &active, "terminal-1"),
    );
    let retired_ref = record_ref("writer-retired-1");
    let retired = admit_retired(
        &authority,
        &active_ref,
        &active,
        &terminal_ref,
        &terminal,
        retired_preimage(&authority, &active_ref, &active, &terminal_ref, &terminal),
    );
    let successor_ref = record_ref("writer-active-2");
    let successor = admit_successor(
        &authority,
        &retired_ref,
        &retired,
        &terminal_ref,
        &terminal,
        successor_preimage(&authority, &retired_ref, &retired, &terminal_ref, &terminal),
    );
    let (refs, checkpoints) = checkpoint_chain(
        &authority,
        "principal-control-store-1",
        "principal-control-incarnation-1",
        "checkpoint",
        4,
    );
    Fixture {
        physical_trust_store: PrincipalTrustStoreBinding::new(
            ControlStoreIdV1::parse("trust-store-1").expect("physical trust store"),
            ControlIncarnationIdV1::parse("trust-incarnation-1")
                .expect("physical trust incarnation"),
            authority.principal_id.clone(),
        ),
        principal_control_store: AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("principal-control-store-1")
                .expect("logical principal control store"),
            ControlIncarnationIdV1::parse("principal-control-incarnation-1")
                .expect("logical principal control incarnation"),
        ),
        active_ref,
        active,
        terminal_ref,
        terminal,
        retired_ref,
        retired,
        successor_ref,
        successor,
        checkpoint_refs: refs.try_into().expect("four checkpoint refs"),
        checkpoints: checkpoints.try_into().expect("four checkpoints"),
    }
}

#[test]
fn all_three_clean_projection_branches_close_exact_physical_candidates() {
    let fixture = fixture();
    assert_ne!(
        fixture.physical_trust_store.trust_store_id(),
        fixture.principal_control_store.store_id()
    );
    assert_ne!(
        fixture.physical_trust_store.trust_store_incarnation_id(),
        fixture.principal_control_store.incarnation_id()
    );
    let initial = prepare_initial_principal_resident_writer_transition(
        &fixture.physical_trust_store,
        &fixture.principal_control_store,
        (&fixture.active_ref, &fixture.active),
        &fixture.checkpoint_refs[0],
        &fixture.checkpoints[0],
        &fixture.checkpoint_refs[1],
        &fixture.checkpoints[1],
    )
    .expect("initial projection");
    assert_eq!(
        initial.kind(),
        PrincipalResidentWriterTransitionKind::InitialActive
    );
    assert_eq!(initial.binding(), &fixture.physical_trust_store);
    assert_eq!(initial.records().len(), 2);

    let retirement = prepare_clean_principal_resident_writer_retirement(
        &fixture.physical_trust_store,
        &fixture.principal_control_store,
        fixture.retirement_preimage(),
    )
    .expect("retirement projection");
    assert_eq!(
        retirement.kind(),
        PrincipalResidentWriterTransitionKind::CleanRetirement
    );
    assert_eq!(retirement.records().len(), 3);

    let successor = prepare_clean_principal_resident_writer_successor(
        &fixture.physical_trust_store,
        &fixture.principal_control_store,
        fixture.successor_preimage(),
    )
    .expect("successor projection");
    assert_eq!(
        successor.kind(),
        PrincipalResidentWriterTransitionKind::CleanSuccessor
    );
    assert_eq!(successor.records().len(), 2);
}

#[test]
fn physical_trust_and_logical_control_store_axes_cannot_be_swapped() {
    let fixture = fixture();
    let logical_as_physical = PrincipalTrustStoreBinding::new(
        fixture.principal_control_store.store_id().clone(),
        fixture.principal_control_store.incarnation_id().clone(),
        fixture.active.principal_id().clone(),
    );
    assert!(matches!(
        prepare_initial_principal_resident_writer_transition(
            &logical_as_physical,
            &fixture.principal_control_store,
            (&fixture.active_ref, &fixture.active),
            &fixture.checkpoint_refs[0],
            &fixture.checkpoints[0],
            &fixture.checkpoint_refs[1],
            &fixture.checkpoints[1],
        ),
        Err(PrincipalResidentWriterProjectionError::TrustStoreClosure)
    ));

    let physical_as_logical = AuthorityStoreCoordinate::new(
        fixture.physical_trust_store.trust_store_id().clone(),
        fixture
            .physical_trust_store
            .trust_store_incarnation_id()
            .clone(),
    );
    assert!(matches!(
        prepare_initial_principal_resident_writer_transition(
            &fixture.physical_trust_store,
            &physical_as_logical,
            (&fixture.active_ref, &fixture.active),
            &fixture.checkpoint_refs[0],
            &fixture.checkpoints[0],
            &fixture.checkpoint_refs[1],
            &fixture.checkpoints[1],
        ),
        Err(PrincipalResidentWriterProjectionError::CheckpointClosure)
    ));
}

#[test]
fn substituted_writer_and_terminal_references_fail_before_custody() {
    let fixture = fixture();
    let wrong_active_ref = record_ref("wrong-active-ref");
    assert!(matches!(
        prepare_clean_principal_resident_writer_retirement(
            &fixture.physical_trust_store,
            &fixture.principal_control_store,
            CleanPrincipalResidentWriterTransitionPreimage::new(
                (&wrong_active_ref, &fixture.active),
                (&fixture.terminal_ref, &fixture.terminal),
                (&fixture.retired_ref, &fixture.retired),
                (&fixture.checkpoint_refs[1], &fixture.checkpoints[1]),
                (&fixture.checkpoint_refs[2], &fixture.checkpoints[2]),
            )
        ),
        Err(PrincipalResidentWriterProjectionError::ReferenceClosure)
    ));

    let authority = authority("principal-1");
    let alternate_ref = record_ref("terminal-alternate");
    let alternate = admit_terminal(
        &authority,
        &fixture.active_ref,
        &fixture.active,
        terminal_preimage(
            &authority,
            &fixture.active_ref,
            &fixture.active,
            "terminal-alternate",
        ),
    );
    assert!(matches!(
        prepare_clean_principal_resident_writer_retirement(
            &fixture.physical_trust_store,
            &fixture.principal_control_store,
            CleanPrincipalResidentWriterTransitionPreimage::new(
                (&fixture.active_ref, &fixture.active),
                (&alternate_ref, &alternate),
                (&fixture.retired_ref, &fixture.retired),
                (&fixture.checkpoint_refs[1], &fixture.checkpoints[1]),
                (&fixture.checkpoint_refs[2], &fixture.checkpoints[2]),
            )
        ),
        Err(PrincipalResidentWriterProjectionError::ReferenceClosure)
    ));

    let alternate_retired_ref = record_ref("writer-retired-alternate");
    let alternate_retired = admit_retired(
        &authority,
        &fixture.active_ref,
        &fixture.active,
        &alternate_ref,
        &alternate,
        retired_preimage(
            &authority,
            &fixture.active_ref,
            &fixture.active,
            &alternate_ref,
            &alternate,
        ),
    );
    let alternate_successor = admit_successor(
        &authority,
        &alternate_retired_ref,
        &alternate_retired,
        &alternate_ref,
        &alternate,
        successor_preimage(
            &authority,
            &alternate_retired_ref,
            &alternate_retired,
            &alternate_ref,
            &alternate,
        ),
    );
    let alternate_successor_ref = record_ref("writer-active-alternate");
    assert!(matches!(
        prepare_clean_principal_resident_writer_successor(
            &fixture.physical_trust_store,
            &fixture.principal_control_store,
            CleanPrincipalResidentWriterTransitionPreimage::new(
                (&fixture.retired_ref, &fixture.retired),
                (&fixture.terminal_ref, &fixture.terminal),
                (&alternate_successor_ref, &alternate_successor),
                (&fixture.checkpoint_refs[2], &fixture.checkpoints[2]),
                (&fixture.checkpoint_refs[3], &fixture.checkpoints[3]),
            )
        ),
        Err(PrincipalResidentWriterProjectionError::ReferenceClosure)
    ));
}

#[test]
fn checkpoint_store_principal_and_predecessor_substitutions_fail_closed() {
    let fixture = fixture();
    let original_principal = authority("principal-1");
    let cases = [
        checkpoint_chain(
            &original_principal,
            "other-store",
            "principal-control-incarnation-1",
            "other-store-checkpoint",
            2,
        ),
        checkpoint_chain(
            &original_principal,
            "principal-control-store-1",
            "other-incarnation",
            "other-incarnation-checkpoint",
            2,
        ),
        checkpoint_chain(
            &authority("principal-2"),
            "principal-control-store-1",
            "principal-control-incarnation-1",
            "other-principal-checkpoint",
            2,
        ),
    ];
    for (refs, checkpoints) in cases {
        assert_checkpoint_pair_rejected(
            &fixture,
            &refs[0],
            &checkpoints[0],
            &refs[1],
            &checkpoints[1],
        );
    }

    assert_checkpoint_pair_rejected(
        &fixture,
        &record_ref("wrong-checkpoint-ref"),
        &fixture.checkpoints[0],
        &fixture.checkpoint_refs[1],
        &fixture.checkpoints[1],
    );

    let mut alternate_preimage = checkpoint_preimage(
        1,
        "principal-control-store-1",
        "principal-control-incarnation-1",
        &original_principal,
    );
    alternate_preimage.state_root_sha256 = artifact('9');
    let alternate_predecessor = admit_initial_checkpoint(alternate_preimage);
    let alternate_successor = admit_checkpoint_successor(
        &fixture.checkpoint_refs[0],
        &alternate_predecessor,
        checkpoint_preimage(
            2,
            "principal-control-store-1",
            "principal-control-incarnation-1",
            &original_principal,
        ),
    );
    assert_checkpoint_pair_rejected(
        &fixture,
        &fixture.checkpoint_refs[0],
        &fixture.checkpoints[0],
        &fixture.checkpoint_refs[1],
        &alternate_successor,
    );

    assert_checkpoint_pair_rejected(
        &fixture,
        &fixture.checkpoint_refs[0],
        &fixture.checkpoints[0],
        &fixture.checkpoint_refs[2],
        &fixture.checkpoints[2],
    );

    let mut wrong_domain_predecessor_preimage = checkpoint_preimage(
        1,
        "principal-control-store-1",
        "principal-control-incarnation-1",
        &original_principal,
    );
    wrong_domain_predecessor_preimage.checkpoint_domain_id =
        ControlCheckpointDomainIdV1::parse("another-domain").expect("other domain");
    let wrong_domain_predecessor = admit_initial_checkpoint(wrong_domain_predecessor_preimage);
    let mut wrong_domain_successor_preimage = checkpoint_preimage(
        2,
        "principal-control-store-1",
        "principal-control-incarnation-1",
        &original_principal,
    );
    wrong_domain_successor_preimage.checkpoint_domain_id =
        ControlCheckpointDomainIdV1::parse("another-domain").expect("other domain");
    let wrong_domain_successor = admit_checkpoint_successor(
        &fixture.checkpoint_refs[0],
        &wrong_domain_predecessor,
        wrong_domain_successor_preimage,
    );
    assert_checkpoint_pair_rejected(
        &fixture,
        &fixture.checkpoint_refs[0],
        &wrong_domain_predecessor,
        &fixture.checkpoint_refs[1],
        &wrong_domain_successor,
    );
}

/// Assert that one admitted checkpoint substitution cannot reach custody.
fn assert_checkpoint_pair_rejected(
    fixture: &Fixture,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ControlStateCheckpointV1,
    successor_ref: &ResidentRecordRefV1,
    successor: &ControlStateCheckpointV1,
) {
    assert!(matches!(
        prepare_initial_principal_resident_writer_transition(
            &fixture.physical_trust_store,
            &fixture.principal_control_store,
            (&fixture.active_ref, &fixture.active),
            predecessor_ref,
            predecessor,
            successor_ref,
            successor,
        ),
        Err(PrincipalResidentWriterProjectionError::CheckpointClosure)
    ));
}
