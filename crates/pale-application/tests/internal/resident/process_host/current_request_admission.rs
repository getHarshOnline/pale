//! Current-request evidence and policy admission laws.

use super::*;
use crate::resident::process_host::current_request::*;
use pale_core::records::{DeploymentAcceptanceRevocationEpochV1, PrincipalRevocationEpochV1};

fn current_evidence(
    current: &crate::resident::AmbientSessionAdmissionContext<'_>,
) -> ResidentCurrentRequestEvidence {
    let grant = current.grant.as_ref().expect("fixture grant");
    let capability = current
        .capability_revocations
        .as_ref()
        .expect("fixture capability frontier");
    ResidentCurrentRequestEvidence {
        grant: ResidentCurrentGrantEvidence {
            grant_ref: grant.grant_ref.clone(),
            grant: grant.grant.clone(),
            issued_trust_checkpoint_ref: grant.issued_trust_checkpoint_ref.clone(),
            issued_trust_checkpoint: grant.issued_trust_checkpoint.clone(),
        },
        capability_revocations: ResidentCurrentCapabilityRevocationEvidence {
            checkpoint_ref: capability.checkpoint_ref.clone(),
            checkpoint: capability.checkpoint.clone(),
            grant_head: capability
                .grant_head
                .map(|head| ResidentCurrentGrantRevocationEvidence {
                    revocation_ref: head.revocation_ref.clone(),
                    revocation: head.revocation.clone(),
                }),
        },
        deployment_revocations: ResidentCurrentDeploymentRevocationEvidence {
            epoch: current.deployment_revocations.epoch,
            binding_head: current.deployment_revocations.binding_head.map(|head| {
                ResidentCurrentDeploymentRevocationHeadEvidence {
                    revocation_ref: head.revocation_ref.clone(),
                    revocation: head.revocation.clone(),
                }
            }),
        },
        session_revocations: ResidentCurrentSessionRevocationEvidence {
            epoch: current.session_revocations.epoch,
            revocation_ref: current.session_revocations.revocation_ref.cloned(),
            revocation_sha256: current.session_revocations.revocation_sha256.cloned(),
        },
    }
}

fn with_current_request(
    suffix: &str,
    run: impl FnOnce(
        &crate::resident::AdmittedAmbientSession,
        &crate::resident::AmbientSessionAdmissionContext<'_>,
        &CommonRequestEnvelopeV1,
        ResidentCurrentRequestEvidence,
    ),
) {
    let (generation, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        suffix,
        |_| {},
        |semantic, request, _, _| {
            run(
                semantic.session,
                semantic.current,
                request,
                current_evidence(semantic.current),
            );
        },
    );
}

#[test]
fn current_evidence_derives_session_authority_without_copying_the_request() {
    with_current_request(
        "current-evidence-exact",
        |session, current, request, evidence| {
            let admitted = evidence
                .session_admission(session, current.now)
                .expect("the exact current evidence joins the admitted session");
            let admitted = admitted.as_record();
            assert_eq!(
                &admitted.session_admission_id,
                session.lease().session_admission_id()
            );
            assert_eq!(
                &admitted.session_admission_lease_digest,
                session.lease().session_admission_lease_digest()
            );
            assert_eq!(&admitted.session_grant_id, session.grant_id());
            assert_eq!(
                &admitted.deployment_binding_ref,
                session.lease().deployment_binding_ref()
            );
            assert_eq!(
                &admitted.checked_revocation_checkpoint_ref,
                &evidence.capability_revocations.checkpoint_ref
            );
            assert_eq!(
                admitted.checked_revocation_checkpoint_sha256.as_str(),
                evidence
                    .capability_revocations
                    .checkpoint
                    .canonical_checkpoint_sha256()
                    .as_str()
            );
            assert_eq!(admitted.checked_at, current.now);
            assert_ne!(
                &admitted.checked_revocation_checkpoint_ref,
                &request
                    .session_admission()
                    .checked_revocation_checkpoint_ref,
                "the fixture request carries a different claimed frontier; current evidence must win"
            );
        },
    );
}

#[test]
fn request_session_fields_cannot_replace_substituted_grant_evidence() {
    with_current_request(
        "current-evidence-grant-substitution",
        |session, current, request, mut evidence| {
            evidence.grant.grant_ref =
                crate::local_operation_start_fixture::record_ref("substituted-grant");
            assert!(matches!(
                evidence.session_admission(session, current.now),
                Err(ResidentCurrentRequestEvidenceError::Mismatch(
                    ResidentCurrentRequestEvidenceMismatch::Grant
                ))
            ));
            assert_eq!(
                request.session_admission().session_grant_id,
                *session.grant_id()
            );
        },
    );
}

#[test]
fn issued_trust_and_capability_frontier_substitution_are_distinct_failures() {
    with_current_request(
        "current-evidence-trust-substitution",
        |session, current, _, evidence| {
            let mut wrong_trust = evidence.clone();
            wrong_trust.grant.issued_trust_checkpoint_ref =
                crate::local_operation_start_fixture::record_ref("substituted-issued-trust");
            assert!(matches!(
                wrong_trust.session_admission(session, current.now),
                Err(ResidentCurrentRequestEvidenceError::Mismatch(
                    ResidentCurrentRequestEvidenceMismatch::IssuedTrust
                ))
            ));

            let mut wrong_capability = evidence;
            wrong_capability.capability_revocations.checkpoint_ref =
                crate::local_operation_start_fixture::record_ref("substituted-capability-frontier");
            assert!(matches!(
                wrong_capability.session_admission(session, current.now),
                Err(ResidentCurrentRequestEvidenceError::Mismatch(
                    ResidentCurrentRequestEvidenceMismatch::CapabilityCheckpoint
                ))
            ));
        },
    );
}

#[test]
fn stale_deployment_and_session_frontiers_are_distinct_failures() {
    with_current_request(
        "current-evidence-stale-frontiers",
        |session, current, _, evidence| {
            let mut stale_deployment = evidence.clone();
            stale_deployment.deployment_revocations.epoch =
                DeploymentAcceptanceRevocationEpochV1::new(
                    stale_deployment.deployment_revocations.epoch.get() + 1,
                )
                .expect("next deployment epoch");
            assert!(matches!(
                stale_deployment.session_admission(session, current.now),
                Err(ResidentCurrentRequestEvidenceError::Mismatch(
                    ResidentCurrentRequestEvidenceMismatch::DeploymentRevocation
                ))
            ));

            let mut stale_session = evidence;
            stale_session.session_revocations.epoch =
                PrincipalRevocationEpochV1::new(stale_session.session_revocations.epoch.get() + 1)
                    .expect("next session epoch");
            assert!(matches!(
                stale_session.session_admission(session, current.now),
                Err(ResidentCurrentRequestEvidenceError::Mismatch(
                    ResidentCurrentRequestEvidenceMismatch::SessionRevocation
                ))
            ));
        },
    );
}
