//! Shared immutable terms for deployment proposal and final admission.
//!
//! The record carries the exact scoped deployment intent and hashes every
//! owner-resolved policy dependency. Its self-digest makes the complete value
//! reusable across later signed protocol stages without allowing those stages
//! to reinterpret the terms.

use super::*;
use serde::Serialize;

/// Public schema root for shared deployment terms.
pub const DEPLOYMENT_TERMS_V1_SCHEMA_ID: &str = "deployment-terms.v1";
/// Wire schema carried by every shared deployment-terms value.
pub const DEPLOYMENT_TERMS_V1_SCHEMA: &str = "pale/deployment-terms.v1";

resident_integrity_record!(
    /// Complete immutable terms shared by deployment proposal and final binding.
    DeploymentTermsV1,
    DeploymentTermsWireV1,
    DeploymentTermsPreimageV1,
    canonical_deployment_terms_sha256: DeploymentTermsDigest,
    DEPLOYMENT_TERMS_V1_SCHEMA,
    "deployment terms",
    {
        /// Portable workspace whose exact revision is being deployed.
        workspace_id: WorkspaceId,
        /// Exact admitted workspace revision.
        workspace_commit: WorkspaceCommitV1,
        /// Stable identity of the scoped deployment.
        deployment_id: DeploymentIdV1,
        /// Immutable distribution release selected by the workspace compiler.
        distribution_release: DistributionReleaseV1,
        /// Canonical identity of the selected distribution release bytes.
        distribution_release_sha256: DistributionReleaseDigest,
        /// Principal under whose resident authority the deployment operates.
        principal_scope: ResidentPrincipalIdV1,
        /// Controller whose policy ceiling bounds the deployment.
        controller_scope: ResidentControllerIdV1,
        /// Optional relationship ceiling; absence does not infer a relationship.
        relationship_scope: Option<RelationshipScopeV1>,
        /// Selected deployment-environment identity.
        environment: DeploymentEnvironmentV1,
        /// Canonical identity of the exact environment-ceiling bytes.
        environment_sha256: DeploymentEnvironmentDigest,
        /// Immutable processing-role reference.
        processing_role_ref: ResidentRecordRefV1,
        /// Canonical identity of the resolved processing-role bytes.
        processing_role_sha256: ProcessingRoleDigest,
        /// Immutable confidentiality-policy reference.
        confidentiality_policy_ref: ResidentRecordRefV1,
        /// Canonical identity of the resolved confidentiality-policy bytes.
        confidentiality_policy_sha256: ConfidentialityPolicyDigest,
        /// Effective classification, which may only narrow every owner ceiling.
        data_classification: DataClassificationV1,
        /// Explicit purposes admitted by the resolved role and policy.
        permitted_purposes: Vec<PermittedPurposeV1>
    },
    validate_deployment_terms_shape,
    wire_transform = deployment_terms_wire_schema
);

impl DeploymentTermsV1 {
    /// Admit complete deployment terms after resolving their workspace,
    /// release, environment, role, policy, classification, and purpose laws.
    ///
    /// `resolve_terms` owns those external semantic checks. This record does
    /// not infer policy from names and does not treat a reference/digest pair
    /// as sufficient authority merely because its shape is valid.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when schema or canonical identity
    /// validation fails, or when `resolve_terms` rejects any owner-resolved
    /// deployment invariant.
    pub fn admit_contextual_wire<F>(
        wire: DeploymentTermsWireV1,
        expected_schema_sha256: &SchemaDigest,
        resolve_terms: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(&DeploymentTermsPreimageV1) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, _digest| {
            if preimage.schema_sha256 != *expected_schema_sha256 {
                return Err(InvalidResidentRecordV1::SchemaCoordinate {
                    family: "deployment terms",
                });
            }
            resolve_terms(preimage)
        })
    }
}

fn validate_deployment_terms_shape(
    value: &DeploymentTermsPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted(
        "deployment terms permitted purposes",
        &value.permitted_purposes,
    )
}

fn deployment_terms_wire_schema(schema: &mut schemars::Schema) {
    project_resident_set_property_v1(schema, "permitted_purposes", None);
}
