//! Historical finalization readmission over exact in-memory physical evidence.

use super::*;
use crate::resident::principal_checkpoint::tests::finalization::submission;
use crate::resident::{deployment_finalization::recovery::*, record_projection};
use pale_ports::{
    ControlCheckpointHead, ControlRecord, ControlRecordBytes, ControlRecordDigest,
    DeploymentFinalizationRecords, DeploymentFinalizationReplayIdentity,
    DeploymentFinalizationSnapshot, PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding,
};
use std::cell::Cell;

#[derive(Clone, Copy)]
pub(in crate::resident) enum StoredChange {
    None,
    CandidateOrder,
    ReceiptIssuer,
    CommitAtExpiry,
    ReplayKey,
    AuthorityBytes,
    RuntimeBytes,
}

pub(in crate::resident) fn with_evidence(
    change: StoredChange,
    test: impl FnOnce(
        DeploymentFinalizationSnapshot,
        &AdmittedDeploymentFinalization<'_>,
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        &SchemaDigest,
        &SchemaDigest,
        &LocalAdminAuthorityV1,
    ),
) {
    with_evidence_at_store(
        change,
        "deployment-store-1",
        "deployment-incarnation-1",
        test,
    );
}

pub(in crate::resident) fn with_evidence_at_store(
    change: StoredChange,
    store_id: &str,
    incarnation_id: &str,
    test: impl FnOnce(
        DeploymentFinalizationSnapshot,
        &AdmittedDeploymentFinalization<'_>,
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        &SchemaDigest,
        &SchemaDigest,
        &LocalAdminAuthorityV1,
    ),
) {
    with_evidence_for_deployment(
        change,
        store_id,
        incarnation_id,
        "deployment-binding-finalization-1",
        "runtime-finalization-1",
        test,
    );
}

pub(in crate::resident) fn with_evidence_for_deployment(
    change: StoredChange,
    store_id: &str,
    incarnation_id: &str,
    binding_reference: &str,
    runtime_reference: &str,
    test: impl FnOnce(
        DeploymentFinalizationSnapshot,
        &AdmittedDeploymentFinalization<'_>,
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        &SchemaDigest,
        &SchemaDigest,
        &LocalAdminAuthorityV1,
    ),
) {
    submission::with_recovery_submission_for_deployment(
        70,
        store_id,
        incarnation_id,
        binding_reference,
        runtime_reference,
        |constructed, context, predecessor_control| {
            let mut command_value = submission::command(&constructed, context);
            command_value["expires_at"] = 75.into();
            let submission = DeploymentFinalizationSubmission::admit(
                constructed,
                submission::wire(command_value),
                context,
                |_, _, _, _, _| Ok(()),
            )
            .unwrap();
            let finalization = submission.construction().finalization();
            let successor_control = submission.construction().checkpoint();
            let command_ref = record_ref("deployment-finalization-command");
            let receipt_ref = record_ref("deployment-finalization-receipt");
            let receipt_id =
                LocalAdminReceiptIdV1::parse("deployment-finalization-receipt").unwrap();
            let receipt_schema = SchemaDigest::parse("8".repeat(64)).unwrap();
            let committed_at = UnixTimeV1::new(if matches!(change, StoredChange::CommitAtExpiry) {
                75
            } else {
                74
            })
            .unwrap();
            let recorded_at = committed_at;
            let receipt_projection =
            crate::resident::deployment_finalization::projection::FinalizationReceiptProjection::new(
                finalization,
                successor_control,
                &command_ref,
                submission.command(),
            )
            .unwrap();
            let selected = finalization.records();
            let runtime = selected.runtime_authority.1;
            let receipt = LocalAdminReceiptV1::issue_contextual(
                LocalAdminReceiptPreimageV1 {
                    schema_id: SchemaId::parse(LOCAL_ADMIN_RECEIPT_V1_SCHEMA).unwrap(),
                    schema_sha256: receipt_schema.clone(),
                    admin_receipt_id: receipt_id.clone(),
                    admin_command_ref: command_ref.clone(),
                    admin_command_sha256: submission
                        .command()
                        .canonical_admin_command_sha256()
                        .clone(),
                    disposition: LocalAdminDispositionV1::Committed,
                    committed_records: receipt_projection.committed_records().to_vec(),
                    head_transitions: receipt_projection.head_transitions().to_vec(),
                    failure_code: None,
                    committed_at: Some(committed_at),
                    recorded_at,
                    deployment_runtime_authority_ref: selected.runtime_authority.0.clone(),
                    deployment_runtime_authority_sha256: runtime
                        .canonical_deployment_runtime_authority_sha256()
                        .clone(),
                    issuer_principal_id: runtime.principal_id().clone(),
                    issuer_key_id: runtime.subject_key_id().clone(),
                    issuer_key_epoch: *runtime.subject_key_epoch(),
                },
                &receipt_projection.admission_context(
                    finalization,
                    &command_ref,
                    submission.command(),
                    &receipt_id,
                    &receipt_schema,
                    &committed_at,
                    &recorded_at,
                ),
                |_, _, _| Ok(test_signature()),
            )
            .unwrap();
            let substituted_receipt: Option<LocalAdminReceiptWireV1> =
                if matches!(change, StoredChange::ReceiptIssuer) {
                    Some(crate::deployment_finalization_fixture::changed_wire(
                        &receipt,
                        "canonical_admin_receipt_sha256",
                        |value| value["issuer_key_id"] = "substituted-runtime-key".into(),
                    ))
                } else {
                    None
                };

            let mut command = submission.command().clone();
            if matches!(change, StoredChange::CandidateOrder) {
                let mut value = submission::command(submission.construction(), context);
                value["expires_at"] = 75.into();
                value["candidate_records"]
                    .as_array_mut()
                    .unwrap()
                    .swap(0, 1);
                command = LocalAdminCommandV1::admit_historical_authenticated_wire(
                    submission::wire(value),
                    &HistoricalLocalAdminCommandAdmissionContextV1 {
                        expected_schema_sha256: context.expected_schema_sha256,
                        trust_checkpoint: context.trust_checkpoint,
                        admin_authority_ref: context.admin_authority_ref,
                        admin_authority: context.admin_authority,
                        committed_at,
                    },
                    |_, _, _, _, _| Ok(()),
                )
                .unwrap();
            }
            let command_coordinate = record_projection::coordinate(
                &command_ref,
                command.canonical_admin_command_sha256().as_str(),
            )
            .unwrap();
            let authority_coordinate = record_projection::coordinate(
                selected.trust.predecessor.local_admin_authority_ref(),
                context
                    .admin_authority
                    .canonical_admin_authority_sha256()
                    .as_str(),
            )
            .unwrap();
            let mut authority_record =
                record_projection::record(authority_coordinate.clone(), context.admin_authority)
                    .unwrap();
            if matches!(change, StoredChange::AuthorityBytes) {
                authority_record = substituted_body(authority_record);
            }
            let mut runtime_record = record_projection::record(
                record_projection::coordinate(
                    selected.runtime_authority.0,
                    runtime
                        .canonical_deployment_runtime_authority_sha256()
                        .as_str(),
                )
                .unwrap(),
                runtime,
            )
            .unwrap();
            if matches!(change, StoredChange::RuntimeBytes) {
                runtime_record = substituted_body(runtime_record);
            }
            let receipt_record = if let Some(wire) = substituted_receipt {
                let value = serde_json::to_value(&wire).unwrap();
                let digest = value["canonical_admin_receipt_sha256"]
                    .as_str()
                    .expect("receipt digest");
                record_projection::record_with_schema(
                    record_projection::coordinate(&receipt_ref, digest).unwrap(),
                    SchemaId::parse(LOCAL_ADMIN_RECEIPT_V1_SCHEMA).unwrap(),
                    receipt_schema.clone(),
                    &wire,
                )
                .unwrap()
            } else {
                record_projection::record_with_schema(
                    record_projection::coordinate(
                        &receipt_ref,
                        receipt.canonical_admin_receipt_sha256().as_str(),
                    )
                    .unwrap(),
                    receipt.schema_id().clone(),
                    receipt.schema_sha256().clone(),
                    &receipt,
                )
                .unwrap()
            };
            let records = DeploymentFinalizationRecords::new(
                record_projection::record(command_coordinate.clone(), &command).unwrap(),
                record_projection::record(
                    record_projection::coordinate(
                        selected.binding.0,
                        selected.binding.1.canonical_binding_sha256().as_str(),
                    )
                    .unwrap(),
                    selected.binding.1,
                )
                .unwrap(),
                record_projection::record(
                    record_projection::coordinate(
                        selected.checkpoint_signer.0,
                        selected
                            .checkpoint_signer
                            .1
                            .canonical_checkpoint_signing_authority_sha256()
                            .as_str(),
                    )
                    .unwrap(),
                    selected.checkpoint_signer.1,
                )
                .unwrap(),
                runtime_record,
                record_projection::record(
                    record_projection::coordinate(
                        selected.trust.successor_ref,
                        selected
                            .trust
                            .successor
                            .canonical_trust_checkpoint_sha256()
                            .as_str(),
                    )
                    .unwrap(),
                    selected.trust.successor,
                )
                .unwrap(),
                record_projection::record(
                    record_projection::coordinate(
                        successor_control.0,
                        successor_control
                            .1
                            .canonical_control_checkpoint_sha256()
                            .as_str(),
                    )
                    .unwrap(),
                    successor_control.1,
                )
                .unwrap(),
                receipt_record,
            )
            .unwrap();
            let replay = DeploymentFinalizationReplayIdentity::new(
                command.admin_command_id().clone(),
                if matches!(change, StoredChange::ReplayKey) {
                    LocalAdminIdempotencyKeyV1::parse("substituted-replay-key").unwrap()
                } else {
                    command.idempotency_key().clone()
                },
                authority_coordinate,
                command_coordinate,
                receipt.admin_receipt_id().clone(),
            );
            let predecessor_trust_head =
                trust_head(selected.trust.predecessor_ref, selected.trust.predecessor);
            let successor_trust_head =
                trust_head(selected.trust.successor_ref, selected.trust.successor);
            let predecessor_control_head =
                control_head(predecessor_control.0, predecessor_control.1);
            let successor_control_head = control_head(successor_control.0, successor_control.1);
            let evidence = DeploymentFinalizationSnapshot::new(
                PrincipalTrustStoreBinding::new(
                    selected.trust.predecessor.trust_store_id().clone(),
                    selected
                        .trust
                        .predecessor
                        .trust_store_incarnation_id()
                        .clone(),
                    selected.trust.predecessor.principal_id().clone(),
                ),
                replay,
                authority_record,
                records,
                predecessor_trust_head,
                record_projection::record(
                    record_projection::coordinate(
                        selected.trust.predecessor_ref,
                        selected
                            .trust
                            .predecessor
                            .canonical_trust_checkpoint_sha256()
                            .as_str(),
                    )
                    .unwrap(),
                    selected.trust.predecessor,
                )
                .unwrap(),
                successor_trust_head,
                predecessor_control_head,
                record_projection::record(
                    record_projection::coordinate(
                        predecessor_control.0,
                        predecessor_control
                            .1
                            .canonical_control_checkpoint_sha256()
                            .as_str(),
                    )
                    .unwrap(),
                    predecessor_control.1,
                )
                .unwrap(),
                successor_control_head,
                submission.construction().accumulator().clone(),
                submission
                    .construction()
                    .accumulator()
                    .binding()
                    .transaction()
                    .successor(),
                ArtifactDigest::parse("e".repeat(64)).unwrap(),
            )
            .unwrap();
            test(
                evidence,
                finalization,
                predecessor_control,
                successor_control,
                context.expected_schema_sha256,
                &receipt_schema,
                context.admin_authority,
            );
        },
    );
}

fn basis<'a, 'records>(
    finalization: &'a AdmittedDeploymentFinalization<'records>,
    predecessor_control: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    successor_control: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    command_schema: &'a SchemaDigest,
    receipt_schema: &'a SchemaDigest,
) -> HistoricalDeploymentFinalizationRecoveryBasis<'a, 'records> {
    HistoricalDeploymentFinalizationRecoveryBasis {
        finalization,
        predecessor_trust: (
            finalization.records().trust.predecessor_ref,
            finalization.records().trust.predecessor,
        ),
        successor_trust: (
            finalization.records().trust.successor_ref,
            finalization.records().trust.successor,
        ),
        predecessor_control,
        successor_control,
        command_schema_sha256: command_schema,
        receipt_schema_sha256: receipt_schema,
    }
}

fn recover(
    evidence: DeploymentFinalizationSnapshot,
    basis: HistoricalDeploymentFinalizationRecoveryBasis<'_, '_>,
    authority: &LocalAdminAuthorityV1,
    command_calls: &Cell<usize>,
    receipt_calls: &Cell<usize>,
) -> Result<RecoveredHistoricalDeploymentFinalization, HistoricalDeploymentFinalizationRecoveryError>
{
    let mut resolver = TestAuthorityResolver {
        outcome: TestAuthorityOutcome::Resolved(authority),
    };
    let mut command_verifier = TestCommandSignatureVerifier {
        calls: command_calls,
        reject: false,
    };
    let mut receipt_verifier = TestReceiptSignatureVerifier {
        calls: receipt_calls,
        reject: false,
    };
    recover_historical_deployment_finalization(
        evidence,
        basis,
        &mut resolver,
        &mut command_verifier,
        &mut receipt_verifier,
    )
}

#[derive(Clone, Copy)]
enum TestAuthorityOutcome<'a> {
    Resolved(&'a LocalAdminAuthorityV1),
    Unavailable,
    Conflicting,
}

struct TestAuthorityResolver<'a> {
    outcome: TestAuthorityOutcome<'a>,
}

impl HistoricalLocalAdminAuthorityResolver for TestAuthorityResolver<'_> {
    fn resolve_historical_local_admin_authority(
        &mut self,
        _: &ControlRecord,
        _: &PrincipalTrustCheckpointV1,
    ) -> HistoricalLocalAdminAuthorityResolution {
        match self.outcome {
            TestAuthorityOutcome::Resolved(authority) => {
                HistoricalLocalAdminAuthorityResolution::Resolved(Box::new(authority.clone()))
            }
            TestAuthorityOutcome::Unavailable => {
                HistoricalLocalAdminAuthorityResolution::Unavailable
            }
            TestAuthorityOutcome::Conflicting => {
                HistoricalLocalAdminAuthorityResolution::Conflicting
            }
        }
    }
}

struct TestCommandSignatureVerifier<'a> {
    calls: &'a Cell<usize>,
    reject: bool,
}

impl HistoricalLocalAdminCommandSignatureVerifier for TestCommandSignatureVerifier<'_> {
    fn verify_historical_local_admin_command_signature(
        &mut self,
        _: &Ed25519PublicKeyV1,
        _: &[u8],
        _: &LocalAdminCommandPreimageV1,
        _: &LocalAdminCommandDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.calls.set(self.calls.get() + 1);
        if self.reject {
            Err(rejected_signature())
        } else {
            Ok(())
        }
    }
}

struct TestReceiptSignatureVerifier<'a> {
    calls: &'a Cell<usize>,
    reject: bool,
}

impl HistoricalLocalAdminReceiptSignatureVerifier for TestReceiptSignatureVerifier<'_> {
    fn verify_historical_local_admin_receipt_signature(
        &mut self,
        _: &Ed25519PublicKeyV1,
        _: &[u8],
        _: &LocalAdminReceiptPreimageV1,
        _: &LocalAdminReceiptDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.calls.set(self.calls.get() + 1);
        if self.reject {
            Err(rejected_signature())
        } else {
            Ok(())
        }
    }
}

#[test]
fn expired_at_restart_command_recovers_at_its_authenticated_commit_time() {
    with_evidence(
        StoredChange::None,
        |evidence,
         finalization,
         predecessor_control,
         successor_control,
         command_schema,
         receipt_schema,
         authority| {
            let expected_transaction = evidence.committed_transaction();
            let command_calls = Cell::new(0);
            let receipt_calls = Cell::new(0);
            let recovered = recover(
                evidence,
                basis(
                    finalization,
                    predecessor_control,
                    successor_control,
                    command_schema,
                    receipt_schema,
                ),
                authority,
                &command_calls,
                &receipt_calls,
            )
            .unwrap();
            assert_eq!(
                recovered.evidence().committed_transaction(),
                expected_transaction
            );
            assert_eq!(recovered.command().expires_at().get(), 75);
            assert_eq!(recovered.receipt().committed_at().unwrap().get(), 74);
            assert_eq!((command_calls.get(), receipt_calls.get()), (1, 1));
        },
    );
}

#[test]
fn unknown_or_conflicting_historical_authority_fails_before_signatures() {
    for conflicting in [false, true] {
        with_evidence(
            StoredChange::None,
            |evidence,
             finalization,
             predecessor_control,
             successor_control,
             command_schema,
             receipt_schema,
             _| {
                let command_calls = Cell::new(0);
                let receipt_calls = Cell::new(0);
                let mut resolver = TestAuthorityResolver {
                    outcome: if conflicting {
                        TestAuthorityOutcome::Conflicting
                    } else {
                        TestAuthorityOutcome::Unavailable
                    },
                };
                let mut command_verifier = TestCommandSignatureVerifier {
                    calls: &command_calls,
                    reject: false,
                };
                let mut receipt_verifier = TestReceiptSignatureVerifier {
                    calls: &receipt_calls,
                    reject: false,
                };
                let result = recover_historical_deployment_finalization(
                    evidence,
                    basis(
                        finalization,
                        predecessor_control,
                        successor_control,
                        command_schema,
                        receipt_schema,
                    ),
                    &mut resolver,
                    &mut command_verifier,
                    &mut receipt_verifier,
                );
                if conflicting {
                    assert!(matches!(
                    result,
                    Err(HistoricalDeploymentFinalizationRecoveryError::HistoricalAuthorityConflict)
                ));
                } else {
                    assert!(matches!(
                    result,
                    Err(HistoricalDeploymentFinalizationRecoveryError::HistoricalAuthorityUnavailable)
                ));
                }
                assert_eq!((command_calls.get(), receipt_calls.get()), (0, 0));
            },
        );
    }
}

#[test]
fn substituted_physical_or_semantic_evidence_never_reaches_both_signatures() {
    for change in [
        StoredChange::CandidateOrder,
        StoredChange::ReceiptIssuer,
        StoredChange::CommitAtExpiry,
        StoredChange::ReplayKey,
        StoredChange::AuthorityBytes,
        StoredChange::RuntimeBytes,
    ] {
        with_evidence(
            change,
            |evidence,
             finalization,
             predecessor_control,
             successor_control,
             command_schema,
             receipt_schema,
             authority| {
                let command_calls = Cell::new(0);
                let receipt_calls = Cell::new(0);
                let result = recover(
                    evidence,
                    basis(
                        finalization,
                        predecessor_control,
                        successor_control,
                        command_schema,
                        receipt_schema,
                    ),
                    authority,
                    &command_calls,
                    &receipt_calls,
                );
                assert!(result.is_err());
                assert_eq!(receipt_calls.get(), 0);
            },
        );
    }
}

#[test]
fn signature_failure_and_wrong_historical_head_are_typed_failures() {
    with_evidence(
        StoredChange::None,
        |evidence,
         finalization,
         predecessor_control,
         successor_control,
         command_schema,
         receipt_schema,
         authority| {
            let command_calls = Cell::new(0);
            let receipt_calls = Cell::new(0);
            let mut resolver = TestAuthorityResolver {
                outcome: TestAuthorityOutcome::Resolved(authority),
            };
            let mut rejected_command = TestCommandSignatureVerifier {
                calls: &command_calls,
                reject: true,
            };
            let mut receipt_verifier = TestReceiptSignatureVerifier {
                calls: &receipt_calls,
                reject: false,
            };
            let result = recover_historical_deployment_finalization(
                evidence.clone(),
                basis(
                    finalization,
                    predecessor_control,
                    successor_control,
                    command_schema,
                    receipt_schema,
                ),
                &mut resolver,
                &mut rejected_command,
                &mut receipt_verifier,
            );
            assert!(matches!(
                result,
                Err(HistoricalDeploymentFinalizationRecoveryError::Record(_))
            ));

            let mut resolver = TestAuthorityResolver {
                outcome: TestAuthorityOutcome::Resolved(authority),
            };
            let mut command_verifier = TestCommandSignatureVerifier {
                calls: &command_calls,
                reject: false,
            };
            let mut rejected_receipt = TestReceiptSignatureVerifier {
                calls: &receipt_calls,
                reject: true,
            };
            let result = recover_historical_deployment_finalization(
                evidence.clone(),
                basis(
                    finalization,
                    predecessor_control,
                    successor_control,
                    command_schema,
                    receipt_schema,
                ),
                &mut resolver,
                &mut command_verifier,
                &mut rejected_receipt,
            );
            assert!(matches!(
                result,
                Err(HistoricalDeploymentFinalizationRecoveryError::Record(_))
            ));

            let mut resolver = TestAuthorityResolver {
                outcome: TestAuthorityOutcome::Resolved(authority),
            };
            let mut command_verifier = TestCommandSignatureVerifier {
                calls: &command_calls,
                reject: false,
            };
            let result = recover_historical_deployment_finalization(
                evidence,
                HistoricalDeploymentFinalizationRecoveryBasis {
                    finalization,
                    predecessor_trust: (
                        finalization.records().trust.successor_ref,
                        finalization.records().trust.successor,
                    ),
                    successor_trust: (
                        finalization.records().trust.successor_ref,
                        finalization.records().trust.successor,
                    ),
                    predecessor_control,
                    successor_control,
                    command_schema_sha256: command_schema,
                    receipt_schema_sha256: receipt_schema,
                },
                &mut resolver,
                &mut command_verifier,
                &mut receipt_verifier,
            );
            assert!(matches!(
                result,
                Err(HistoricalDeploymentFinalizationRecoveryError::Association)
            ));
        },
    );
}

fn rejected_signature() -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "test signature",
        invariant: "accepted test signature",
    }
}

fn test_signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("test signature")
}

fn substituted_body(record: ControlRecord) -> ControlRecord {
    ControlRecord::new(
        record.coordinate().clone(),
        record.schema_id().clone(),
        record.schema_sha256().clone(),
        ControlRecordBytes::new(b"{}".to_vec()).unwrap(),
    )
}

fn trust_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> PrincipalTrustCheckpointHead {
    PrincipalTrustCheckpointHead::new(
        checkpoint.principal_id().clone(),
        *checkpoint.checkpoint_epoch(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_trust_checkpoint_sha256().as_str(),
        )
        .unwrap(),
    )
}

fn control_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        pale_ports::ControlRecordCoordinate::new(
            reference.clone(),
            ControlRecordDigest::parse(checkpoint.canonical_control_checkpoint_sha256().as_str())
                .unwrap(),
        ),
    )
}
