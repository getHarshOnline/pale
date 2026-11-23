//! Exact finalization joins over real record admission and synthetic signatures.
//! Existing head history is a declared test model, not native custody evidence.

use super::principal_trust_test_support::signed_wire;
use super::resident::*;
use super::session_admission_fixture::{
    deployment_binding_named, principal_key, record_ref, runtime_authority_in_store_named,
};
use pale_core::records::*;
use serde::{de::DeserializeOwned, Serialize};

/// Reseal changed test bytes so rejection exercises meaning, not a stale digest.
pub(crate) fn changed_wire<R: Serialize, W: DeserializeOwned>(
    record: &R,
    digest_field: &str,
    change: impl FnOnce(&mut serde_json::Value),
) -> W {
    let mut value = serde_json::to_value(record).unwrap();
    value.as_object_mut().unwrap().remove(digest_field);
    value.as_object_mut().unwrap().remove("signature");
    change(&mut value);
    let canonical = pale_core::canon::to_canonical_string(&value).unwrap();
    let digest = ArtifactDigest::hash_bytes(canonical.as_bytes());
    signed_wire(&value, digest_field, &digest)
}

pub(crate) struct Fixture {
    pub(crate) prior_ref: ResidentRecordRefV1,
    pub(crate) prior: PrincipalTrustCheckpointV1,
    pub(crate) next_ref: ResidentRecordRefV1,
    pub(crate) next: PrincipalTrustCheckpointV1,
    pub(crate) binding_ref: ResidentRecordRefV1,
    pub(crate) binding: DeploymentBindingV1,
    pub(crate) signer_ref: ResidentRecordRefV1,
    pub(crate) signer: CheckpointSigningAuthorityV1,
    pub(crate) runtime_ref: ResidentRecordRefV1,
    pub(crate) runtime: DeploymentRuntimeAuthorityV1,
    pub(crate) checkpoint_heads: Vec<CheckpointSigningAuthorityHeadV1>,
    pub(crate) runtime_heads: Vec<DeploymentRuntimeAuthorityHeadV1>,
}

impl Fixture {
    pub(crate) fn from_trust(
        prior_ref: &ResidentRecordRefV1,
        prior: &PrincipalTrustCheckpointV1,
        authority_ref: &ResidentRecordRefV1,
        authority: &CheckpointSigningAuthorityV1,
    ) -> Self {
        Self::from_trust_named(prior_ref, prior, authority_ref, authority, "1")
    }

    pub(crate) fn from_trust_named(
        prior_ref: &ResidentRecordRefV1,
        prior: &PrincipalTrustCheckpointV1,
        authority_ref: &ResidentRecordRefV1,
        authority: &CheckpointSigningAuthorityV1,
        identity: &str,
    ) -> Self {
        Self::from_trust_named_at_store(
            (prior_ref, prior),
            (authority_ref, authority),
            identity,
            &format!("deployment-store-{identity}"),
            &format!("deployment-incarnation-{identity}"),
            &format!("deployment-binding-finalization-{identity}"),
            &format!("runtime-finalization-{identity}"),
        )
    }

    pub(crate) fn from_trust_named_at_store(
        (prior_ref, prior): (&ResidentRecordRefV1, &PrincipalTrustCheckpointV1),
        (authority_ref, authority): (&ResidentRecordRefV1, &CheckpointSigningAuthorityV1),
        identity: &str,
        store_id: &str,
        incarnation_id: &str,
        binding_reference: &str,
        runtime_reference: &str,
    ) -> Self {
        let checkpoint_heads = vec![authority.active_head(authority_ref.clone()).unwrap()];
        let binding_ref = record_ref(binding_reference);
        let (binding, _) = deployment_binding_named((30, 200), identity);
        let runtime = runtime_authority_in_store_named(
            &binding_ref,
            &binding,
            (40, 180),
            &[
                CAPABILITY_RESULT_V1_SCHEMA,
                COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
                DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
                LOCAL_ADMIN_RECEIPT_V1_SCHEMA,
                OPERATION_HANDLE_V1_SCHEMA,
                OPERATION_OBSERVATION_V1_SCHEMA,
                SESSION_ADMISSION_LEASE_V1_SCHEMA,
            ],
            store_id,
            incarnation_id,
            identity,
        );
        let signer_ref = record_ref(&format!("deployment-checkpoint-signer-{identity}"));
        let profile_ref = record_ref(&format!("deployment-checkpoint-profile-{identity}"));
        let profile_digest = CheckpointProfileDigest::parse("a".repeat(64)).unwrap();
        let signer_wire = changed_wire(
            authority,
            "canonical_checkpoint_signing_authority_sha256",
            |value| {
                value["checkpoint_signing_authority_id"] =
                    format!("deployment-checkpoint-signer-{identity}").into();
                value["control_store_id"] = runtime.control_store_id().as_str().into();
                value["control_incarnation_id"] = runtime.control_incarnation_id().as_str().into();
                value["scope"] = serde_json::to_value(CheckpointSigningScopeV1::Deployment {
                    deployment_binding_ref: binding_ref.clone(),
                    deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
                })
                .unwrap();
                value["checkpoint_profile_ref"] = profile_ref.as_str().into();
                value["checkpoint_profile_sha256"] = profile_digest.as_str().into();
                value["issued_at"] = 40.into();
                value["expires_at"] = 180.into();
            },
        );
        let signer = CheckpointSigningAuthorityV1::admit_initial_deployment_wire(
            signer_wire,
            &DeploymentCheckpointSigningAdmissionContextV1 {
                expected_schema_sha256: authority.schema_sha256(),
                deployment_binding_ref: &binding_ref,
                deployment_binding: &binding,
                control_store_id: runtime.control_store_id(),
                control_incarnation_id: runtime.control_incarnation_id(),
                checkpoint_profile_ref: &profile_ref,
                checkpoint_profile_sha256: &profile_digest,
                signer: principal_key(prior.principal_id(), prior.issuer_key_id()),
            },
            |_| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .unwrap();
        let mut fixture = Self {
            prior_ref: prior_ref.clone(),
            prior: prior.clone(),
            next_ref: record_ref(&format!("trust-finalization-{identity}")),
            next: prior.clone(),
            binding_ref,
            binding,
            signer_ref,
            signer,
            runtime_ref: record_ref(runtime_reference),
            runtime,
            checkpoint_heads,
            runtime_heads: Vec::new(),
        };
        fixture.refresh_successor(|_| {});
        fixture
    }

    pub(crate) fn refresh_successor(&mut self, change: impl FnOnce(&mut serde_json::Value)) {
        let mut signers = self.checkpoint_heads.clone();
        signers.push(self.signer.active_head(self.signer_ref.clone()).unwrap());
        signers.sort_by(|a, b| {
            a.checkpoint_signing_authority_id
                .cmp(&b.checkpoint_signing_authority_id)
        });
        let mut runtimes = self.runtime_heads.clone();
        runtimes.push(self.runtime.active_head(self.runtime_ref.clone()).unwrap());
        runtimes.sort_by(|a, b| a.runtime_authority_id.cmp(&b.runtime_authority_id));
        let signer_root = checkpoint_signing_authority_heads_root_sha256(&signers).unwrap();
        let runtime_root = deployment_runtime_authority_heads_root_sha256(&runtimes).unwrap();
        let wire = changed_wire(&self.prior, "canonical_trust_checkpoint_sha256", |value| {
            value["trust_checkpoint_id"] = self.next_ref.as_str().into();
            value["checkpoint_epoch"] = (self.prior.checkpoint_epoch().get() + 1).into();
            value["previous_trust_checkpoint_ref"] = self.prior_ref.as_str().into();
            value["previous_trust_checkpoint_sha256"] = self
                .prior
                .canonical_trust_checkpoint_sha256()
                .as_str()
                .into();
            value["issued_at"] = 60.into();
            value["checkpoint_signing_authority_heads_root_sha256"] = signer_root.as_str().into();
            value["deployment_runtime_authority_heads_root_sha256"] = runtime_root.as_str().into();
            change(value);
        });
        self.next = PrincipalTrustCheckpointV1::admit_successor_wire(
            wire,
            &self.prior_ref,
            &self.prior,
            self.prior.schema_sha256(),
            |_, _| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .unwrap();
    }

    pub(crate) fn candidate(&self) -> DeploymentFinalizationCandidate<'_> {
        DeploymentFinalizationCandidate {
            binding: (&self.binding_ref, &self.binding),
            checkpoint_signer: (&self.signer_ref, &self.signer),
            runtime_authority: (&self.runtime_ref, &self.runtime),
            trust: AdmittedPrincipalTrustCheckpointTransition::new(
                &self.prior_ref,
                &self.prior,
                &self.next_ref,
                &self.next,
            )
            .unwrap(),
            checkpoint_heads: &self.checkpoint_heads,
            runtime_heads: &self.runtime_heads,
        }
    }
}
