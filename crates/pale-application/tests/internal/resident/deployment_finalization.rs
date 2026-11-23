//! Exact finalization joins over real record admission and synthetic signatures.
//! Existing head history is a declared test model, not native custody evidence.

use super::*;
use crate::deployment_finalization_fixture::{changed_wire, Fixture};
use crate::principal_trust_test_support::admitted_bootstrap_fixture;
use crate::session_admission_fixture::{record_ref, runtime_authority_in_store};

#[path = "deployment_finalization/recovery.rs"]
pub(in crate::resident) mod recovery;

#[test]
fn exact_insertions_preserve_other_deployments_and_retain_the_record_join() {
    let fixture = Fixture::new();
    let admitted = AdmittedDeploymentFinalization::admit(fixture.candidate()).unwrap();
    assert_eq!(
        admitted.records().binding,
        (&fixture.binding_ref, &fixture.binding)
    );
    assert_eq!(
        admitted.records().checkpoint_heads,
        fixture.checkpoint_heads
    );
    assert_eq!(admitted.records().runtime_heads, fixture.runtime_heads);
    assert_eq!(
        admitted.checkpoint_signer_head(),
        &fixture
            .signer
            .active_head(fixture.signer_ref.clone())
            .unwrap()
    );
    assert_eq!(
        admitted.runtime_authority_head(),
        &fixture
            .runtime
            .active_head(fixture.runtime_ref.clone())
            .unwrap()
    );
}

#[test]
fn missing_substituted_duplicate_and_unordered_old_heads_are_rejected() {
    for case in 0..5 {
        let mut fixture = Fixture::new();
        match case {
            0 => {
                fixture.checkpoint_heads.pop();
            }
            1 => fixture.runtime_heads.clear(),
            2 => fixture.checkpoint_heads[0].authority_ref = record_ref("substituted"),
            3 => fixture
                .checkpoint_heads
                .push(fixture.checkpoint_heads[0].clone()),
            4 => fixture.checkpoint_heads.swap(0, 1),
            _ => unreachable!(),
        }
        assert!(
            AdmittedDeploymentFinalization::admit(fixture.candidate()).is_err(),
            "case {case}"
        );
    }
}

#[test]
fn finalization_cannot_change_unrelated_authority_or_the_root_signer() {
    let changes = [
        (
            "principal_record_ref",
            serde_json::json!("other-principal-record"),
        ),
        ("principal_record_sha256", serde_json::json!("f".repeat(64))),
        ("recovery_policy_ref", serde_json::json!("other-recovery")),
        ("recovery_policy_sha256", serde_json::json!("f".repeat(64))),
        (
            "local_admin_authority_ref",
            serde_json::json!("other-admin"),
        ),
        (
            "local_admin_authority_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        (
            "producer_incarnation_authority_ref",
            serde_json::json!("other-producer-authority"),
        ),
        (
            "producer_incarnation_authority_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        (
            "producer_incarnation_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        ("session_floor_epoch", serde_json::json!(2)),
        (
            "owner_record_signing_authority_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        (
            "owner_signer_publication_frontier_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        (
            "deployment_acceptance_revocation_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        (
            "federation_space_home_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        (
            "node_delegation_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        (
            "node_delegation_revocation_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        ("issuer_key_id", serde_json::json!("other-root-key")),
        ("issuer_key_epoch", serde_json::json!(2)),
        (
            "checkpoint_signing_authority_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
        (
            "deployment_runtime_authority_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
        ),
    ];
    for (field, value) in changes {
        let mut fixture = Fixture::new();
        fixture.refresh_successor(|next| {
            next[field] = value;
        });
        assert!(
            matches!(
                AdmittedDeploymentFinalization::admit(fixture.candidate()),
                Err(DeploymentFinalizationError::Successor)
            ),
            "field {field}"
        );
    }
}

#[test]
fn reused_references_lineages_and_already_selected_deployments_conflict() {
    for case in 0..6 {
        let mut fixture = Fixture::new();
        match case {
            0 => fixture.runtime_ref = fixture.signer_ref.clone(),
            1 => fixture.binding_ref = fixture.prior_ref.clone(),
            2 => fixture.next_ref = fixture.checkpoint_heads[0].authority_ref.clone(),
            3 => {
                fixture.runtime_heads[0].runtime_authority_id =
                    fixture.runtime.runtime_authority_id().clone();
                fixture.refresh_prior_roots();
            }
            4 => {
                fixture.runtime_heads[0].deployment_binding_sha256 =
                    fixture.binding.canonical_binding_sha256().clone();
                fixture.refresh_prior_roots();
            }
            5 => {
                fixture.checkpoint_heads[1].scope = fixture.signer.scope().clone();
                fixture.refresh_prior_roots();
            }
            _ => unreachable!(),
        }
        // A changed predecessor must be paired with its own generic successor;
        // the roots can remain wrong because the conflict must be caught first.
        if case >= 3 {
            fixture.next = PrincipalTrustCheckpointV1::admit_successor_wire(
                changed_wire(
                    &fixture.next,
                    "canonical_trust_checkpoint_sha256",
                    |value| {
                        value["checkpoint_epoch"] =
                            (fixture.prior.checkpoint_epoch().get() + 1).into();
                        value["previous_trust_checkpoint_ref"] = fixture.prior_ref.as_str().into();
                        value["previous_trust_checkpoint_sha256"] = fixture
                            .prior
                            .canonical_trust_checkpoint_sha256()
                            .as_str()
                            .into();
                    },
                ),
                &fixture.prior_ref,
                &fixture.prior,
                fixture.prior.schema_sha256(),
                |_, _| Ok(()),
                |_, _, _, _| Ok(()),
            )
            .unwrap();
        }
        assert!(
            AdmittedDeploymentFinalization::admit(fixture.candidate()).is_err(),
            "case {case}"
        );
    }
}

#[test]
fn unlike_binding_store_and_validity_are_not_joined() {
    for case in 0..6 {
        let mut fixture = Fixture::new();
        match case {
            0 => fixture.binding_ref = record_ref("other-binding"),
            1 => {
                fixture.runtime = runtime_authority_in_store(
                    &fixture.binding_ref,
                    &fixture.binding,
                    (40, 180),
                    &[DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA],
                    "other-store",
                    "other-incarnation",
                )
            }
            2 => fixture.refresh_successor(|next| {
                next["issued_at"] = 180.into();
            }),
            3 => {
                fixture.runtime = runtime_authority_in_store(
                    &fixture.binding_ref,
                    &fixture.binding,
                    (70, 180),
                    &[DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA],
                    "deployment-store",
                    "deployment-incarnation",
                )
            }
            4 => {
                fixture.runtime = runtime_authority_in_store(
                    &fixture.binding_ref,
                    &fixture.binding,
                    (40, 59),
                    &[DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA],
                    "deployment-store",
                    "deployment-incarnation",
                )
            }
            5 => {
                let (bootstrap, _) = admitted_bootstrap_fixture();
                fixture.signer = bootstrap
                    .admitted
                    .atom_view()
                    .authorities()
                    .checkpoint_authority()
                    .clone();
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                AdmittedDeploymentFinalization::admit(fixture.candidate()),
                Err(DeploymentFinalizationError::Scope)
            ),
            "case {case}"
        );
    }
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let (bootstrap, _) = admitted_bootstrap_fixture();
        let atom = bootstrap.admitted.atom_view();
        let (prior_ref, prior) = atom.trust_checkpoint_entry();
        let authorities = atom.authorities();
        let mut fixture = Self::from_trust(
            prior_ref,
            prior,
            authorities.checkpoint_authority_ref(),
            authorities.checkpoint_authority(),
        );
        // Retain an unrelated deployment in both head sets to exercise preservation.
        let mut old_signer = fixture
            .signer
            .active_head(record_ref("other-signer-record"))
            .unwrap();
        old_signer.checkpoint_signing_authority_id =
            CheckpointSigningAuthorityIdV1::parse("other-signer").unwrap();
        old_signer.scope = CheckpointSigningScopeV1::Deployment {
            deployment_binding_ref: record_ref("other-binding"),
            deployment_binding_sha256: DeploymentBindingDigest::parse("e".repeat(64)).unwrap(),
        };
        fixture.checkpoint_heads.push(old_signer);
        let mut old_runtime = fixture
            .runtime
            .active_head(record_ref("other-runtime-record"))
            .unwrap();
        old_runtime.runtime_authority_id =
            DeploymentRuntimeAuthorityIdV1::parse("other-runtime").unwrap();
        old_runtime.deployment_binding_sha256 =
            DeploymentBindingDigest::parse("e".repeat(64)).unwrap();
        fixture.runtime_heads.push(old_runtime);
        fixture.refresh_prior_roots();
        fixture.refresh_successor(|_| {});
        fixture
    }

    pub(crate) fn refresh_prior_roots(&mut self) {
        let signer_root =
            checkpoint_signing_authority_heads_root_sha256(&self.checkpoint_heads).unwrap();
        let runtime_root =
            deployment_runtime_authority_heads_root_sha256(&self.runtime_heads).unwrap();
        let prior_ref = record_ref(&format!(
            "prior-trust-{}",
            self.prior.checkpoint_epoch().get() + 1
        ));
        let successor = PrincipalTrustCheckpointV1::admit_successor_wire(
            changed_wire(&self.prior, "canonical_trust_checkpoint_sha256", |value| {
                value["trust_checkpoint_id"] = prior_ref.as_str().into();
                value["checkpoint_epoch"] = (self.prior.checkpoint_epoch().get() + 1).into();
                value["previous_trust_checkpoint_ref"] = self.prior_ref.as_str().into();
                value["previous_trust_checkpoint_sha256"] = self
                    .prior
                    .canonical_trust_checkpoint_sha256()
                    .as_str()
                    .into();
                value["checkpoint_signing_authority_heads_root_sha256"] =
                    signer_root.as_str().into();
                value["deployment_runtime_authority_heads_root_sha256"] =
                    runtime_root.as_str().into();
            }),
            &self.prior_ref,
            &self.prior,
            self.prior.schema_sha256(),
            |_, _| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .unwrap();
        self.prior_ref = prior_ref;
        self.prior = successor;
    }
}
