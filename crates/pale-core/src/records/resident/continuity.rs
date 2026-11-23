//! Portable resource-continuity declarations and semantic admission.
//!
//! A [`ResourceContinuityProfileV1`] declares desired continuity policy for
//! one [`WorkspaceResourceId`]. It neither stores owner-native observations
//! nor performs any mechanism. The profile owns the exact subject-containment
//! and closed mechanism-coverage laws.
//!
//! These laws do not make reader, proposal-writer, and effect-writer grants
//! substitutable. Nor do they turn a replica, view, cache, archive, transport,
//! session relation, recovery target, or cutover reference into authority or
//! observed native truth.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for resource-continuity profile wires.
pub const RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA_ID: &str = "resource-continuity-profile.v1";
/// Wire schema carried by every resource-continuity profile.
pub const RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA: &str = "pale/resource-continuity-profile.v1";
/// Wire version carried by every resource-continuity profile.
pub const RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA_VERSION: u32 = 1;

resident_id!(
    #[doc = "Stable identity of one resource-continuity profile lineage."]
    ResourceContinuityProfileIdV1,
    "resource continuity profile id"
);
resident_id!(
    #[doc = "Stable identity of one declared resource copy or projection."]
    ResourceContinuityCopyIdV1,
    "resource continuity copy id"
);
resident_coordinate!(
    /// Recovery-time objective in whole seconds.
    ///
    /// Zero expresses an immediate target, not proof that recovery is possible.
    RecoveryTimeObjectiveSecondsV1,
    "recovery time objective seconds",
    zero_aware
);
resident_coordinate!(
    /// Recovery-point objective in whole seconds.
    ///
    /// Zero expresses a no-loss target, not proof that the target is met.
    RecoveryPointObjectiveSecondsV1,
    "recovery point objective seconds",
    zero_aware
);

/// Exact immutable record coordinate used by a continuity declaration.
///
/// The pair prevents a mutable locator or same-name replacement from acting as
/// the declared contract. It does not prove authority, availability, or current
/// applicability of the referenced record.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ContinuityRecordCoordinateV1 {
    /// Immutable resident or owner-native record reference.
    record_ref: ResidentRecordRefV1,
    /// SHA-256 of the exact referenced bytes.
    record_sha256: ArtifactDigest,
}

impl ContinuityRecordCoordinateV1 {
    /// Bind one immutable reference to its exact bytes.
    #[must_use]
    pub const fn new(record_ref: ResidentRecordRefV1, record_sha256: ArtifactDigest) -> Self {
        Self {
            record_ref,
            record_sha256,
        }
    }

    /// Borrow the immutable record reference.
    #[must_use]
    pub const fn record_ref(&self) -> &ResidentRecordRefV1 {
        &self.record_ref
    }

    /// Borrow the digest of the exact referenced bytes.
    #[must_use]
    pub const fn record_sha256(&self) -> &ArtifactDigest {
        &self.record_sha256
    }
}

/// Native owner plus exact evidence for one authority axis.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ContinuityAuthorityBindingV1 {
    /// Native system retaining the authority.
    owner_system_id: OwnerSystemIdV1,
    /// Exact immutable authority evidence.
    authority: AuthorityEvidenceV1,
}

impl ContinuityAuthorityBindingV1 {
    /// Bind one native owner to exact immutable authority evidence.
    #[must_use]
    pub const fn new(owner_system_id: OwnerSystemIdV1, authority: AuthorityEvidenceV1) -> Self {
        Self {
            owner_system_id,
            authority,
        }
    }

    /// Borrow the native system retaining this authority axis.
    #[must_use]
    pub const fn owner_system_id(&self) -> &OwnerSystemIdV1 {
        &self.owner_system_id
    }

    /// Borrow the exact immutable authority evidence.
    #[must_use]
    pub const fn authority(&self) -> &AuthorityEvidenceV1 {
        &self.authority
    }
}

/// Controller identity plus exact evidence for its declared authority.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ContinuityControllerBindingV1 {
    /// Controller determining purpose, policy, and lifecycle.
    controller_id: ResidentControllerIdV1,
    /// Exact immutable evidence for that controller authority.
    authority: ContinuityRecordCoordinateV1,
}

impl ContinuityControllerBindingV1 {
    /// Bind one controller identity to exact authority evidence.
    #[must_use]
    pub const fn new(
        controller_id: ResidentControllerIdV1,
        authority: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self {
            controller_id,
            authority,
        }
    }

    /// Borrow the controller identity.
    #[must_use]
    pub const fn controller_id(&self) -> &ResidentControllerIdV1 {
        &self.controller_id
    }

    /// Borrow the exact controller-authority coordinate.
    #[must_use]
    pub const fn authority(&self) -> &ContinuityRecordCoordinateV1 {
        &self.authority
    }
}

/// Independently typed authority axes for one logical resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityAuthorityV1 {
    /// Native system owning the resource rather than Pale.
    native_owner: OwnerSystemIdV1,
    /// Exact record defining the selected authority model.
    authority_model: ContinuityRecordCoordinateV1,
    /// Exact owner and authority for the authoritative source.
    authoritative_source: ContinuityAuthorityBindingV1,
    /// Controller determining purpose, policy, and lifecycle.
    controller: ContinuityControllerBindingV1,
    /// Exact owner and authority for consequential native effects.
    effect_authority: ContinuityAuthorityBindingV1,
}

impl ResourceContinuityAuthorityV1 {
    /// Declare all authority axes without transferring any of them to Pale.
    #[must_use]
    pub const fn new(
        native_owner: OwnerSystemIdV1,
        authority_model: ContinuityRecordCoordinateV1,
        authoritative_source: ContinuityAuthorityBindingV1,
        controller: ContinuityControllerBindingV1,
        effect_authority: ContinuityAuthorityBindingV1,
    ) -> Self {
        Self {
            native_owner,
            authority_model,
            authoritative_source,
            controller,
            effect_authority,
        }
    }

    /// Borrow the native resource owner.
    #[must_use]
    pub const fn native_owner(&self) -> &OwnerSystemIdV1 {
        &self.native_owner
    }

    /// Borrow the selected authority-model coordinate.
    #[must_use]
    pub const fn authority_model(&self) -> &ContinuityRecordCoordinateV1 {
        &self.authority_model
    }

    /// Borrow the authoritative-source binding.
    #[must_use]
    pub const fn authoritative_source(&self) -> &ContinuityAuthorityBindingV1 {
        &self.authoritative_source
    }

    /// Borrow the controller binding.
    #[must_use]
    pub const fn controller(&self) -> &ContinuityControllerBindingV1 {
        &self.controller
    }

    /// Borrow the consequential-effect authority binding.
    #[must_use]
    pub const fn effect_authority(&self) -> &ContinuityAuthorityBindingV1 {
        &self.effect_authority
    }
}

/// Subject class admitted by a resource-continuity access declaration.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(
    tag = "kind",
    content = "identity",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ResourceContinuitySubjectV1 {
    /// Sovereign principal admitted by an external trust decision.
    Principal(ResidentPrincipalIdV1),
    /// Controller admitted for the resource's declared purpose and lifecycle.
    Controller(ResidentControllerIdV1),
}

/// Non-substitutable access roles for a resource-continuity subject.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceContinuityAccessRoleV1 {
    /// Read the governed representation through its owner-defined path.
    Reader,
    /// Propose a mutation without authority to cause the native effect.
    ProposalWriter,
    /// Request a consequential mutation under separately resolved authority.
    EffectWriter,
}

/// Explicitly absent or exactly bound optional contract.
///
/// This algebra avoids treating a missing JSON member or `null` as a selected
/// transport or credential contract.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContinuityOptionalContractV1 {
    /// No contract is selected for this axis.
    Absent {},
    /// The axis is bound to one exact immutable contract.
    Bound {
        /// Exact selected contract coordinate.
        contract: ContinuityRecordCoordinateV1,
    },
}

/// One role-specific grant and its independently selected attachments.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityGrantV1 {
    /// Principal or controller receiving this grant.
    subject: ResourceContinuitySubjectV1,
    /// Exact non-substitutable access role.
    role: ResourceContinuityAccessRoleV1,
    /// Exact record authorizing this role grant.
    authority: ContinuityRecordCoordinateV1,
    /// Explicit transport-contract disposition for this grant.
    transport: ContinuityOptionalContractV1,
    /// Explicit credential-contract disposition; no credential value is carried.
    credential: ContinuityOptionalContractV1,
}

impl ResourceContinuityGrantV1 {
    /// Declare one role-specific grant without applying it.
    #[must_use]
    pub const fn new(
        subject: ResourceContinuitySubjectV1,
        role: ResourceContinuityAccessRoleV1,
        authority: ContinuityRecordCoordinateV1,
        transport: ContinuityOptionalContractV1,
        credential: ContinuityOptionalContractV1,
    ) -> Self {
        Self {
            subject,
            role,
            authority,
            transport,
            credential,
        }
    }

    /// Borrow the grantee identity and class.
    #[must_use]
    pub const fn subject(&self) -> &ResourceContinuitySubjectV1 {
        &self.subject
    }

    /// Return the non-substitutable access role.
    #[must_use]
    pub const fn role(&self) -> ResourceContinuityAccessRoleV1 {
        self.role
    }

    /// Borrow the exact authority for this role grant.
    #[must_use]
    pub const fn authority(&self) -> &ContinuityRecordCoordinateV1 {
        &self.authority
    }

    /// Borrow the explicit transport-contract disposition.
    #[must_use]
    pub const fn transport(&self) -> &ContinuityOptionalContractV1 {
        &self.transport
    }

    /// Borrow the explicit credential-contract disposition.
    #[must_use]
    pub const fn credential(&self) -> &ContinuityOptionalContractV1 {
        &self.credential
    }
}

/// Canonical authorized-principal set with schema-visible bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ResourceContinuityPrincipalSetV1(
    #[schemars(length(max = 1024))] Vec<ResidentPrincipalIdV1>,
);

impl ResourceContinuityPrincipalSetV1 {
    /// Construct a bounded, strictly ordered principal set.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for duplicate, unordered, or
    /// oversized input.
    pub fn new(values: Vec<ResidentPrincipalIdV1>) -> Result<Self, InvalidResidentRecordV1> {
        strictly_sorted("resource continuity authorized principals", &values)?;
        Ok(Self(values))
    }

    /// Borrow the canonical principal set.
    #[must_use]
    pub fn as_slice(&self) -> &[ResidentPrincipalIdV1] {
        &self.0
    }

    /// Decide whether the set authorizes one principal identity.
    #[must_use]
    pub fn contains(&self, value: &ResidentPrincipalIdV1) -> bool {
        self.0.binary_search(value).is_ok()
    }

    /// Revalidate a structurally decoded set.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        strictly_sorted("resource continuity authorized principals", &self.0)
    }
}

/// Canonical authorized-controller set with schema-visible bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ResourceContinuityControllerSetV1(
    #[schemars(length(max = 1024))] Vec<ResidentControllerIdV1>,
);

impl ResourceContinuityControllerSetV1 {
    /// Construct a bounded, strictly ordered controller set.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for duplicate, unordered, or
    /// oversized input.
    pub fn new(values: Vec<ResidentControllerIdV1>) -> Result<Self, InvalidResidentRecordV1> {
        strictly_sorted("resource continuity authorized controllers", &values)?;
        Ok(Self(values))
    }

    /// Borrow the canonical controller set.
    #[must_use]
    pub fn as_slice(&self) -> &[ResidentControllerIdV1] {
        &self.0
    }

    /// Decide whether the set authorizes one controller identity.
    #[must_use]
    pub fn contains(&self, value: &ResidentControllerIdV1) -> bool {
        self.0.binary_search(value).is_ok()
    }

    /// Revalidate a structurally decoded set.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        strictly_sorted("resource continuity authorized controllers", &self.0)
    }
}

/// Canonical grant set with schema-visible bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ResourceContinuityGrantSetV1(
    #[schemars(length(max = 1024))] Vec<ResourceContinuityGrantV1>,
);

impl ResourceContinuityGrantSetV1 {
    /// Construct grants ordered uniquely by `(subject, role)`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for duplicate, unordered, or
    /// oversized input.
    pub fn new(values: Vec<ResourceContinuityGrantV1>) -> Result<Self, InvalidResidentRecordV1> {
        let value = Self(values);
        value.validate_order()?;
        Ok(value)
    }

    /// Borrow the canonical grant set.
    #[must_use]
    pub fn as_slice(&self) -> &[ResourceContinuityGrantV1] {
        &self.0
    }

    /// Decide whether one subject holds one explicitly declared role.
    #[must_use]
    pub fn contains(
        &self,
        subject: &ResourceContinuitySubjectV1,
        role: ResourceContinuityAccessRoleV1,
    ) -> bool {
        self.0
            .binary_search_by(|grant| (&grant.subject, grant.role).cmp(&(subject, role)))
            .is_ok()
    }

    /// Revalidate canonical ordering and uniqueness.
    fn validate_order(&self) -> Result<(), InvalidResidentRecordV1> {
        bounded_len("resource continuity grants", self.0.len())?;
        if self
            .0
            .windows(2)
            .all(|pair| (&pair[0].subject, pair[0].role) < (&pair[1].subject, pair[1].role))
        {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::NonCanonical {
                relation: "resource continuity grants",
            })
        }
    }
}

/// Authorized subjects and their explicit role grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityAccessV1 {
    /// Principals eligible to appear in [`Self::grants`].
    authorized_principals: ResourceContinuityPrincipalSetV1,
    /// Controllers eligible to appear in [`Self::grants`].
    authorized_controllers: ResourceContinuityControllerSetV1,
    /// Canonical explicit grants; presence in an authorized set grants nothing.
    grants: ResourceContinuityGrantSetV1,
}

impl ResourceContinuityAccessV1 {
    /// Construct access policy and prove subject membership and required roles.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when a collection is noncanonical,
    /// a grantee is unauthorized, or no reader or effect writer is declared.
    pub fn new(
        authorized_principals: ResourceContinuityPrincipalSetV1,
        authorized_controllers: ResourceContinuityControllerSetV1,
        grants: ResourceContinuityGrantSetV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let value = Self {
            authorized_principals,
            authorized_controllers,
            grants,
        };
        value.validate()?;
        Ok(value)
    }

    /// Borrow the authorized principal set.
    #[must_use]
    pub const fn authorized_principals(&self) -> &ResourceContinuityPrincipalSetV1 {
        &self.authorized_principals
    }

    /// Borrow the authorized controller set.
    #[must_use]
    pub const fn authorized_controllers(&self) -> &ResourceContinuityControllerSetV1 {
        &self.authorized_controllers
    }

    /// Borrow the canonical explicit grant set.
    #[must_use]
    pub const fn grants(&self) -> &ResourceContinuityGrantSetV1 {
        &self.grants
    }

    /// Decide whether one subject occurs in its matching authorized set.
    #[must_use]
    pub fn authorizes(&self, subject: &ResourceContinuitySubjectV1) -> bool {
        match subject {
            ResourceContinuitySubjectV1::Principal(id) => self.authorized_principals.contains(id),
            ResourceContinuitySubjectV1::Controller(id) => self.authorized_controllers.contains(id),
        }
    }

    /// Reprove canonicality, membership, and minimum role coverage.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        self.authorized_principals.validate()?;
        self.authorized_controllers.validate()?;
        self.grants.validate_order()?;
        if self
            .grants
            .0
            .iter()
            .any(|grant| !self.authorizes(&grant.subject))
        {
            return Err(continuity_reject(
                "every grant subject must be an authorized principal or controller",
            ));
        }
        for required in [
            ResourceContinuityAccessRoleV1::Reader,
            ResourceContinuityAccessRoleV1::EffectWriter,
        ] {
            if !self.grants.0.iter().any(|grant| grant.role == required) {
                return Err(continuity_reject(
                    "reader and effect-writer roles require independent explicit grants",
                ));
            }
        }
        Ok(())
    }
}

/// Physical or service scope retaining one declared representation.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(
    tag = "kind",
    content = "scope",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ContinuityHolderV1 {
    /// Owner-declared logical device scope.
    Device(ResourceContinuityDeviceScopeIdV1),
    /// Owner-declared logical machine scope.
    Machine(ResourceContinuityMachineScopeIdV1),
    /// Remote service scope expressed as an immutable owner-native reference.
    Service(ResidentRecordRefV1),
    /// Remote residency scope expressed as an immutable owner-native reference.
    Location(ResidentRecordRefV1),
}

/// Semantically distinct retained representation classes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceContinuityCopyClassV1 {
    /// Authorized local replica participating in a declared replica policy.
    LocalReplica,
    /// Authorized remote replica under remote retention and revocation policy.
    RemoteReplica,
    /// Read-oriented derivative that is never source authority.
    View,
    /// Disposable acceleration state reconstructible from an admitted source.
    Cache,
    /// Retained representation under a distinct access and retention class.
    Archive,
}

/// Exact non-substitutable contracts governing one retained representation.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityCopyPolicyV1 {
    /// Exact policy defining the permitted representation scope.
    scope: ContinuityRecordCoordinateV1,
    /// Exact contract resolving the owner-native source frontier.
    frontier_contract: ContinuityRecordCoordinateV1,
    /// Exact retention contract for this representation.
    retention_contract: ContinuityRecordCoordinateV1,
    /// Exact revocation contract for this representation.
    revocation_contract: ContinuityRecordCoordinateV1,
}

impl ResourceContinuityCopyPolicyV1 {
    /// Group the four non-substitutable contracts governing one representation.
    #[must_use]
    pub const fn new(
        scope: ContinuityRecordCoordinateV1,
        frontier_contract: ContinuityRecordCoordinateV1,
        retention_contract: ContinuityRecordCoordinateV1,
        revocation_contract: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self {
            scope,
            frontier_contract,
            retention_contract,
            revocation_contract,
        }
    }

    /// Borrow the permitted representation-scope contract.
    #[must_use]
    pub const fn scope(&self) -> &ContinuityRecordCoordinateV1 {
        &self.scope
    }

    /// Borrow the owner-native frontier contract.
    #[must_use]
    pub const fn frontier_contract(&self) -> &ContinuityRecordCoordinateV1 {
        &self.frontier_contract
    }

    /// Borrow the retention contract.
    #[must_use]
    pub const fn retention_contract(&self) -> &ContinuityRecordCoordinateV1 {
        &self.retention_contract
    }

    /// Borrow the revocation contract.
    #[must_use]
    pub const fn revocation_contract(&self) -> &ContinuityRecordCoordinateV1 {
        &self.revocation_contract
    }
}

/// One declared copy, view, cache, or archive.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityCopyV1 {
    /// Stable copy-declaration identity.
    copy_id: ResourceContinuityCopyIdV1,
    /// Non-substitutable representation class.
    class: ResourceContinuityCopyClassV1,
    /// Logical holder scope, never a machine path.
    holder: ContinuityHolderV1,
    /// Native custodian and exact custody authority.
    custodian: ContinuityAuthorityBindingV1,
    /// Exact scope, frontier, retention, and revocation contracts.
    policy: ResourceContinuityCopyPolicyV1,
}

impl ResourceContinuityCopyV1 {
    /// Declare one retained representation without creating or observing it.
    #[must_use]
    pub const fn new(
        copy_id: ResourceContinuityCopyIdV1,
        class: ResourceContinuityCopyClassV1,
        holder: ContinuityHolderV1,
        custodian: ContinuityAuthorityBindingV1,
        policy: ResourceContinuityCopyPolicyV1,
    ) -> Self {
        Self {
            copy_id,
            class,
            holder,
            custodian,
            policy,
        }
    }

    /// Borrow the stable identity used for canonical ordering.
    #[must_use]
    pub const fn copy_id(&self) -> &ResourceContinuityCopyIdV1 {
        &self.copy_id
    }

    /// Return the non-substitutable representation class.
    #[must_use]
    pub const fn class(&self) -> ResourceContinuityCopyClassV1 {
        self.class
    }

    /// Borrow the logical holder scope.
    #[must_use]
    pub const fn holder(&self) -> &ContinuityHolderV1 {
        &self.holder
    }

    /// Borrow the native custody binding.
    #[must_use]
    pub const fn custodian(&self) -> &ContinuityAuthorityBindingV1 {
        &self.custodian
    }

    /// Borrow the exact representation-policy contracts.
    #[must_use]
    pub const fn policy(&self) -> &ResourceContinuityCopyPolicyV1 {
        &self.policy
    }
}

/// Canonical copy declaration set with schema-visible bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ResourceContinuityCopySetV1(
    #[schemars(length(max = 1024))] Vec<ResourceContinuityCopyV1>,
);

impl ResourceContinuityCopySetV1 {
    /// Construct copies ordered uniquely by [`ResourceContinuityCopyV1::copy_id`].
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for duplicate, unordered,
    /// oversized, or class/scope-inconsistent input.
    pub fn new(values: Vec<ResourceContinuityCopyV1>) -> Result<Self, InvalidResidentRecordV1> {
        let value = Self(values);
        value.validate()?;
        Ok(value)
    }

    /// Borrow the canonical copy declarations.
    #[must_use]
    pub fn as_slice(&self) -> &[ResourceContinuityCopyV1] {
        &self.0
    }

    /// Revalidate canonicality and replica holder classes.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        strictly_sorted_by_key("resource continuity copies", &self.0, |copy| &copy.copy_id)?;
        for copy in &self.0 {
            match (copy.class, &copy.holder) {
                (
                    ResourceContinuityCopyClassV1::LocalReplica,
                    ContinuityHolderV1::Device(_) | ContinuityHolderV1::Machine(_),
                )
                | (
                    ResourceContinuityCopyClassV1::RemoteReplica,
                    ContinuityHolderV1::Service(_) | ContinuityHolderV1::Location(_),
                )
                | (
                    ResourceContinuityCopyClassV1::View
                    | ResourceContinuityCopyClassV1::Cache
                    | ResourceContinuityCopyClassV1::Archive,
                    _,
                ) => {}
                _ => {
                    return Err(continuity_reject(
                        "local and remote replicas require matching holder scopes",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Durable metadata classes selected by a merge protocol.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceConvergenceMetadataV1 {
    /// Deletion marker retained because removal participates in convergence.
    Tombstone,
    /// Writer-specific monotone clock.
    WriterClock,
    /// Causal ordering metadata.
    CausalClock,
    /// Per-writer version-vector frontier.
    VersionVector,
    /// Merge-parent or ancestry identity.
    MergeAncestry,
    /// Field-level source-owner identity.
    PerFieldOrigin,
    /// Unresolved conflict marker retained until its owner rules.
    ConflictMarker,
}

/// Nonempty canonical merge-metadata requirement set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ResourceConvergenceMetadataSetV1(
    #[schemars(length(min = 1, max = 7))] Vec<ResourceConvergenceMetadataV1>,
);

impl ResourceConvergenceMetadataSetV1 {
    /// Construct a nonempty, strictly ordered metadata requirement set.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for empty, duplicate, unordered, or
    /// oversized input.
    pub fn new(
        values: Vec<ResourceConvergenceMetadataV1>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let value = Self(values);
        value.validate()?;
        Ok(value)
    }

    /// Borrow the selected metadata requirements.
    #[must_use]
    pub fn as_slice(&self) -> &[ResourceConvergenceMetadataV1] {
        &self.0
    }

    /// Revalidate nonemptiness and canonical ordering.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        if self.0.is_empty() {
            return Err(continuity_reject(
                "merge protocol requires explicit durable metadata",
            ));
        }
        if self.0.len() > 7 {
            return Err(InvalidResidentRecordV1::BoundExceeded {
                relation: "resource convergence metadata",
                observed: self.0.len(),
                maximum: 7,
            });
        }
        if self.0.windows(2).all(|pair| pair[0] < pair[1]) {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::NonCanonical {
                relation: "resource convergence metadata",
            })
        }
    }
}

/// Exact effect-writer selection for an exclusive-writer policy.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityWriterV1 {
    /// Authorized principal or controller selected as the sole writer.
    subject: ResourceContinuitySubjectV1,
    /// Exact authority for the exclusive-writer selection.
    authority: ContinuityRecordCoordinateV1,
}

impl ResourceContinuityWriterV1 {
    /// Bind an authorized subject to exact writer-selection authority.
    #[must_use]
    pub const fn new(
        subject: ResourceContinuitySubjectV1,
        authority: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self { subject, authority }
    }

    /// Borrow the selected writer subject.
    #[must_use]
    pub const fn subject(&self) -> &ResourceContinuitySubjectV1 {
        &self.subject
    }

    /// Borrow the exact exclusive-writer selection authority.
    #[must_use]
    pub const fn authority(&self) -> &ContinuityRecordCoordinateV1 {
        &self.authority
    }
}

/// Closed convergence algebra for one logical resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResourceConvergenceV1 {
    /// Exactly one subject is selected as effect writer; merge is not admitted.
    ExclusiveWriter {
        /// Selected writer and exact selection authority.
        writer: ResourceContinuityWriterV1,
    },
    /// Concurrency is admitted only through one exact merge protocol.
    MergeProtocol {
        /// Exact deterministic merge-protocol contract.
        protocol: ContinuityRecordCoordinateV1,
        /// Native owner and authority for merge execution.
        merge_owner: ContinuityAuthorityBindingV1,
        /// Native owner and authority for unresolved conflicts.
        conflict_owner: ContinuityAuthorityBindingV1,
        /// Canonical durable metadata required by this protocol.
        durable_metadata: ResourceConvergenceMetadataSetV1,
    },
}

impl ResourceConvergenceV1 {
    /// Reprove mode-specific invariants against the explicit access grants.
    fn validate(&self, access: &ResourceContinuityAccessV1) -> Result<(), InvalidResidentRecordV1> {
        match self {
            Self::ExclusiveWriter { writer } => {
                if access.grants.contains(
                    writer.subject(),
                    ResourceContinuityAccessRoleV1::EffectWriter,
                ) {
                    Ok(())
                } else {
                    Err(continuity_reject(
                        "exclusive writer must hold an explicit effect-writer grant",
                    ))
                }
            }
            Self::MergeProtocol {
                durable_metadata, ..
            } => durable_metadata.validate(),
        }
    }
}

/// Closed continuity mechanisms requiring independent policy decisions.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceContinuityMechanismKindV1 {
    /// Compare and reconcile declared frontiers without transferring authority.
    Sync,
    /// Maintain an authorized copy under a replica protocol.
    Replication,
    /// Move bytes without making transport a live store or authority.
    Transport,
    /// Transfer one bounded representation between owners or scopes.
    ExportImport,
    /// Rebuild a derivative from an admitted source frontier.
    ProjectionRegeneration,
    /// Protect recoverability through an independently verified backup image.
    BackupRestore,
    /// Reconstruct ordered state from a checkpoint and journal.
    CheckpointReplay,
    /// Change long-term retention and access class.
    Archive,
    /// Change the active writer or owner under an explicit cutover contract.
    MigrationCutover,
    /// Repair an interrupted system under its native recovery contract.
    Recovery,
}

impl ResourceContinuityMechanismKindV1 {
    /// Complete canonical initial-v1 mechanism inventory.
    pub const ALL: [Self; 10] = [
        Self::Sync,
        Self::Replication,
        Self::Transport,
        Self::ExportImport,
        Self::ProjectionRegeneration,
        Self::BackupRestore,
        Self::CheckpointReplay,
        Self::Archive,
        Self::MigrationCutover,
        Self::Recovery,
    ];
}

/// Closed policy disposition for one continuity mechanism.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResourceContinuityMechanismDispositionV1 {
    /// The mechanism is outside the admitted profile.
    Forbidden {},
    /// The mechanism is governed by one exact immutable contract.
    Governed {
        /// Exact contract required before the mechanism can be interpreted.
        contract: ContinuityRecordCoordinateV1,
    },
}

/// Explicit policy for exactly one [`ResourceContinuityMechanismKindV1`].
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityMechanismPolicyV1 {
    /// Mechanism governed by this policy row.
    kind: ResourceContinuityMechanismKindV1,
    /// Explicitly forbidden or exactly governed disposition.
    disposition: ResourceContinuityMechanismDispositionV1,
}

impl ResourceContinuityMechanismPolicyV1 {
    /// Forbid one mechanism without inventing a governing contract.
    #[must_use]
    pub const fn forbidden(kind: ResourceContinuityMechanismKindV1) -> Self {
        Self {
            kind,
            disposition: ResourceContinuityMechanismDispositionV1::Forbidden {},
        }
    }

    /// Govern one mechanism through the supplied exact contract.
    #[must_use]
    pub const fn governed(
        kind: ResourceContinuityMechanismKindV1,
        contract: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self {
            kind,
            disposition: ResourceContinuityMechanismDispositionV1::Governed { contract },
        }
    }

    /// Return the governed mechanism kind.
    #[must_use]
    pub const fn kind(&self) -> ResourceContinuityMechanismKindV1 {
        self.kind
    }

    /// Borrow the explicit forbidden-or-governed disposition.
    #[must_use]
    pub const fn disposition(&self) -> &ResourceContinuityMechanismDispositionV1 {
        &self.disposition
    }
}

/// Exact complete mechanism table with schema-visible cardinality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ResourceContinuityMechanismTableV1(
    #[schemars(length(min = 10, max = 10))] Vec<ResourceContinuityMechanismPolicyV1>,
);

impl ResourceContinuityMechanismTableV1 {
    /// Construct the exact canonical ten-mechanism table.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] unless every closed mechanism kind
    /// occurs exactly once in [`ResourceContinuityMechanismKindV1::ALL`] order.
    pub fn new(
        values: Vec<ResourceContinuityMechanismPolicyV1>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let value = Self(values);
        value.validate()?;
        Ok(value)
    }

    /// Borrow the exact canonical mechanism table.
    #[must_use]
    pub fn as_slice(&self) -> &[ResourceContinuityMechanismPolicyV1] {
        &self.0
    }

    /// Reprove exact inventory coverage and order.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        if self.0.len() != ResourceContinuityMechanismKindV1::ALL.len()
            || !self
                .0
                .iter()
                .zip(ResourceContinuityMechanismKindV1::ALL)
                .all(|(rule, expected)| rule.kind == expected)
        {
            return Err(continuity_reject(
                "mechanism table must cover every closed kind exactly once",
            ));
        }
        Ok(())
    }
}

/// Recovery objectives and exact verification/read-back contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityRecoveryV1 {
    /// Maximum target elapsed seconds for service restoration.
    recovery_time_objective_seconds: RecoveryTimeObjectiveSecondsV1,
    /// Maximum target age in seconds of admissible recovered state.
    recovery_point_objective_seconds: RecoveryPointObjectiveSecondsV1,
    /// Exact contract for recovery verification and native read-back.
    verification_contract: ContinuityRecordCoordinateV1,
}

impl ResourceContinuityRecoveryV1 {
    /// Declare distinct recovery targets and exact verification policy.
    #[must_use]
    pub const fn new(
        recovery_time_objective_seconds: RecoveryTimeObjectiveSecondsV1,
        recovery_point_objective_seconds: RecoveryPointObjectiveSecondsV1,
        verification_contract: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self {
            recovery_time_objective_seconds,
            recovery_point_objective_seconds,
            verification_contract,
        }
    }

    /// Return the recovery-time objective in whole seconds.
    #[must_use]
    pub const fn recovery_time_objective_seconds(&self) -> RecoveryTimeObjectiveSecondsV1 {
        self.recovery_time_objective_seconds
    }

    /// Return the recovery-point objective in whole seconds.
    #[must_use]
    pub const fn recovery_point_objective_seconds(&self) -> RecoveryPointObjectiveSecondsV1 {
        self.recovery_point_objective_seconds
    }

    /// Borrow the exact recovery verification/read-back contract.
    #[must_use]
    pub const fn verification_contract(&self) -> &ContinuityRecordCoordinateV1 {
        &self.verification_contract
    }
}

/// Exact governance policies constraining every continuity mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityGovernanceV1 {
    /// Exact privacy policy.
    privacy_policy: ContinuityRecordCoordinateV1,
    /// Exact residency policy.
    residency_policy: ContinuityRecordCoordinateV1,
    /// Exact retention policy.
    retention_policy: ContinuityRecordCoordinateV1,
    /// Exact revocation policy.
    revocation_policy: ContinuityRecordCoordinateV1,
    /// Exact deletion policy.
    deletion_policy: ContinuityRecordCoordinateV1,
    /// Exact legal-hold policy.
    legal_hold_policy: ContinuityRecordCoordinateV1,
}

impl ResourceContinuityGovernanceV1 {
    /// Bind all six governance axes without treating any one as another.
    #[must_use]
    pub const fn new(
        privacy_policy: ContinuityRecordCoordinateV1,
        residency_policy: ContinuityRecordCoordinateV1,
        retention_policy: ContinuityRecordCoordinateV1,
        revocation_policy: ContinuityRecordCoordinateV1,
        deletion_policy: ContinuityRecordCoordinateV1,
        legal_hold_policy: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self {
            privacy_policy,
            residency_policy,
            retention_policy,
            revocation_policy,
            deletion_policy,
            legal_hold_policy,
        }
    }

    /// Borrow the privacy policy.
    #[must_use]
    pub const fn privacy_policy(&self) -> &ContinuityRecordCoordinateV1 {
        &self.privacy_policy
    }

    /// Borrow the residency policy.
    #[must_use]
    pub const fn residency_policy(&self) -> &ContinuityRecordCoordinateV1 {
        &self.residency_policy
    }

    /// Borrow the retention policy.
    #[must_use]
    pub const fn retention_policy(&self) -> &ContinuityRecordCoordinateV1 {
        &self.retention_policy
    }

    /// Borrow the revocation policy.
    #[must_use]
    pub const fn revocation_policy(&self) -> &ContinuityRecordCoordinateV1 {
        &self.revocation_policy
    }

    /// Borrow the deletion policy.
    #[must_use]
    pub const fn deletion_policy(&self) -> &ContinuityRecordCoordinateV1 {
        &self.deletion_policy
    }

    /// Borrow the legal-hold policy.
    #[must_use]
    pub const fn legal_hold_policy(&self) -> &ContinuityRecordCoordinateV1 {
        &self.legal_hold_policy
    }
}

/// Canonical device-scope set with schema-visible bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ResourceContinuityDeviceScopeSetV1(
    #[schemars(length(max = 1024))] Vec<ResourceContinuityDeviceScopeIdV1>,
);

impl ResourceContinuityDeviceScopeSetV1 {
    /// Construct a bounded, strictly ordered device-scope set.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for duplicate, unordered, or
    /// oversized input.
    pub fn new(
        values: Vec<ResourceContinuityDeviceScopeIdV1>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        strictly_sorted("resource continuity device scopes", &values)?;
        Ok(Self(values))
    }

    /// Borrow the canonical logical device scopes.
    #[must_use]
    pub fn as_slice(&self) -> &[ResourceContinuityDeviceScopeIdV1] {
        &self.0
    }

    /// Revalidate a structurally decoded set.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        strictly_sorted("resource continuity device scopes", &self.0)
    }
}

/// Canonical machine-scope set with schema-visible bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ResourceContinuityMachineScopeSetV1(
    #[schemars(length(max = 1024))] Vec<ResourceContinuityMachineScopeIdV1>,
);

impl ResourceContinuityMachineScopeSetV1 {
    /// Construct a bounded, strictly ordered machine-scope set.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for duplicate, unordered, or
    /// oversized input.
    pub fn new(
        values: Vec<ResourceContinuityMachineScopeIdV1>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        strictly_sorted("resource continuity machine scopes", &values)?;
        Ok(Self(values))
    }

    /// Borrow the canonical logical machine scopes.
    #[must_use]
    pub fn as_slice(&self) -> &[ResourceContinuityMachineScopeIdV1] {
        &self.0
    }

    /// Revalidate a structurally decoded set.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        strictly_sorted("resource continuity machine scopes", &self.0)
    }
}

/// Device, machine, and cross-machine bootstrap scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityScopeV1 {
    /// Canonical logical device scopes; never filesystem paths.
    devices: ResourceContinuityDeviceScopeSetV1,
    /// Canonical logical machine scopes; never filesystem paths.
    machines: ResourceContinuityMachineScopeSetV1,
    /// Exact cross-machine bootstrap contract.
    cross_machine_bootstrap: ContinuityRecordCoordinateV1,
    /// Exact cross-machine bootstrap verification contract.
    bootstrap_verification: ContinuityRecordCoordinateV1,
}

impl ResourceContinuityScopeV1 {
    /// Declare canonical device and machine scopes plus bootstrap contracts.
    #[must_use]
    pub const fn new(
        devices: ResourceContinuityDeviceScopeSetV1,
        machines: ResourceContinuityMachineScopeSetV1,
        cross_machine_bootstrap: ContinuityRecordCoordinateV1,
        bootstrap_verification: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self {
            devices,
            machines,
            cross_machine_bootstrap,
            bootstrap_verification,
        }
    }

    /// Borrow the canonical logical device scopes.
    #[must_use]
    pub const fn devices(&self) -> &ResourceContinuityDeviceScopeSetV1 {
        &self.devices
    }

    /// Borrow the canonical logical machine scopes.
    #[must_use]
    pub const fn machines(&self) -> &ResourceContinuityMachineScopeSetV1 {
        &self.machines
    }

    /// Borrow the exact cross-machine bootstrap contract.
    #[must_use]
    pub const fn cross_machine_bootstrap(&self) -> &ContinuityRecordCoordinateV1 {
        &self.cross_machine_bootstrap
    }

    /// Borrow the exact bootstrap-verification contract.
    #[must_use]
    pub const fn bootstrap_verification(&self) -> &ContinuityRecordCoordinateV1 {
        &self.bootstrap_verification
    }

    /// Revalidate both canonical scope sets.
    fn validate(&self) -> Result<(), InvalidResidentRecordV1> {
        self.devices.validate()?;
        self.machines.validate()
    }
}

/// Explicit relation between resource and session continuity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResourceSessionContinuityV1 {
    /// Resource continuity is unrelated to session continuity.
    Unrelated {},
    /// The relation exists only through one exact contract.
    Bound {
        /// Exact contract governing the cross-domain relation.
        relation_contract: ContinuityRecordCoordinateV1,
    },
}

/// Owner-resolved cutover claim plus an exact rollback contract.
///
/// The cutover-state reference is owner-native evidence. This type deliberately
/// does not reproduce a mutable cutover state machine inside Pale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityCutoverV1 {
    /// Exact owner-resolved current cutover-state record.
    cutover_state: ContinuityRecordCoordinateV1,
    /// Exact rollback contract for the selected cutover mechanism.
    rollback_contract: ContinuityRecordCoordinateV1,
}

impl ResourceContinuityCutoverV1 {
    /// Bind owner-resolved cutover evidence to exact rollback policy.
    #[must_use]
    pub const fn new(
        cutover_state: ContinuityRecordCoordinateV1,
        rollback_contract: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self {
            cutover_state,
            rollback_contract,
        }
    }

    /// Borrow the exact owner-resolved cutover-state record.
    #[must_use]
    pub const fn cutover_state(&self) -> &ContinuityRecordCoordinateV1 {
        &self.cutover_state
    }

    /// Borrow the exact rollback contract.
    #[must_use]
    pub const fn rollback_contract(&self) -> &ContinuityRecordCoordinateV1 {
        &self.rollback_contract
    }
}

/// Owners and immutable destination contracts for continuity evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceContinuityEvidenceV1 {
    /// Owner and authority for producing continuity evidence.
    evidence_owner: ContinuityAuthorityBindingV1,
    /// Owner and authority for native read-back.
    read_back_owner: ContinuityAuthorityBindingV1,
    /// Exact immutable evidence destination contract.
    evidence_destination: ContinuityRecordCoordinateV1,
    /// Exact native read-back contract.
    read_back_contract: ContinuityRecordCoordinateV1,
}

impl ResourceContinuityEvidenceV1 {
    /// Declare evidence and read-back custody without producing evidence.
    #[must_use]
    pub const fn new(
        evidence_owner: ContinuityAuthorityBindingV1,
        read_back_owner: ContinuityAuthorityBindingV1,
        evidence_destination: ContinuityRecordCoordinateV1,
        read_back_contract: ContinuityRecordCoordinateV1,
    ) -> Self {
        Self {
            evidence_owner,
            read_back_owner,
            evidence_destination,
            read_back_contract,
        }
    }

    /// Borrow the evidence-production owner binding.
    #[must_use]
    pub const fn evidence_owner(&self) -> &ContinuityAuthorityBindingV1 {
        &self.evidence_owner
    }

    /// Borrow the native read-back owner binding.
    #[must_use]
    pub const fn read_back_owner(&self) -> &ContinuityAuthorityBindingV1 {
        &self.read_back_owner
    }

    /// Borrow the immutable evidence-destination contract.
    #[must_use]
    pub const fn evidence_destination(&self) -> &ContinuityRecordCoordinateV1 {
        &self.evidence_destination
    }

    /// Borrow the exact native read-back contract.
    #[must_use]
    pub const fn read_back_contract(&self) -> &ContinuityRecordCoordinateV1 {
        &self.read_back_contract
    }
}

resident_record!(
    /// Complete portable continuity declaration for one logical resource.
    ///
    /// Admission proves canonical shape, bounded sets, role separation, exact
    /// mechanism coverage, and self-digest integrity. It does not resolve any
    /// referenced authority, establish native freshness, create a copy, restore
    /// data, change a writer, or cause an effect. Profile admission is also not
    /// contextual continuity admission: a later linker must bind a selection to
    /// its exact principal, project, profile, generation, deployment, route,
    /// harness, host, and session coordinates.
    ///
    /// In the first law below, `G` is the admitted grant relation and `P` and
    /// `C` are the authorized principal and controller sets in [`Self::access`];
    /// every grant subject must belong to their union. In the second, `M` is
    /// [`Self::mechanisms`], `K` is the closed mechanism-kind inventory, and
    /// `pi_kind` projects the set of kinds represented by the table. Equality
    /// proves complete, non-duplicated policy coverage. It does not establish
    /// native availability, freshness, or authority.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/continuity-grants.display.mathml"
    ))]
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/continuity-mechanisms.display.mathml"
    ))]
    pub ResourceContinuityProfileV1, ResourceContinuityProfileWireV1,
    ResourceContinuityProfilePreimageV1,
    resource_continuity_profile_digest: ResourceContinuityProfileDigest,
    RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA, RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA_VERSION,
    "resource continuity profile",
    {
        /// Stable identity of this profile lineage.
        profile_id: ResourceContinuityProfileIdV1,
        /// Existing portable logical resource identity; never a machine path.
        resource_id: WorkspaceResourceId,
        /// Independently typed native authority axes.
        authority: ResourceContinuityAuthorityV1,
        /// Authorized subjects and explicit non-substitutable grants.
        access: ResourceContinuityAccessV1,
        /// Canonical permitted copies and derived representations.
        copies: ResourceContinuityCopySetV1,
        /// Exclusive-writer or explicit merge-protocol algebra.
        convergence: ResourceConvergenceV1,
        /// Complete canonical policy table for every distinct mechanism.
        mechanisms: ResourceContinuityMechanismTableV1,
        /// Distinct recovery objectives and verification contract.
        recovery: ResourceContinuityRecoveryV1,
        /// Privacy, residency, retention, revocation, deletion, and legal-hold policy.
        governance: ResourceContinuityGovernanceV1,
        /// Device, machine, and cross-machine bootstrap scope.
        scope: ResourceContinuityScopeV1,
        /// Explicitly unrelated or exactly bound session-continuity relation.
        session_relation: ResourceSessionContinuityV1,
        /// Owner-resolved cutover evidence and rollback contract.
        cutover: ResourceContinuityCutoverV1,
        /// Evidence/read-back owners and immutable destination contracts.
        evidence: ResourceContinuityEvidenceV1
    },
    validate_resource_continuity_profile
);

impl ResourceContinuityProfileV1 {
    /// Admit a construction preimage under the exact generated schema identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for a schema mismatch, noncanonical
    /// or incomplete declaration, invalid cross-field state, or canonicalization
    /// failure.
    pub fn admit(
        preimage: ResourceContinuityProfilePreimageV1,
        expected_schema_sha256: &SchemaDigest,
    ) -> Result<Self, InvalidResidentRecordV1> {
        if preimage.schema_sha256 != *expected_schema_sha256 {
            return Err(InvalidResidentRecordV1::SchemaCoordinate {
                family: "resource continuity profile",
            });
        }
        Self::seal(preimage)
    }

    /// Admit an untrusted structural wire under the exact generated schema.
    ///
    /// Structural deserialization proves only field shape. This method reproves
    /// semantic invariants and self-digest integrity; referenced authority and
    /// native state remain separate owner operations.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] under the same conditions as
    /// [`Self::admit`], and when the claimed digest differs from the admitted
    /// canonical preimage.
    pub fn admit_wire(
        wire: ResourceContinuityProfileWireV1,
        expected_schema_sha256: &SchemaDigest,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let value = Self {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            profile_id: wire.profile_id,
            resource_id: wire.resource_id,
            authority: wire.authority,
            access: wire.access,
            copies: wire.copies,
            convergence: wire.convergence,
            mechanisms: wire.mechanisms,
            recovery: wire.recovery,
            governance: wire.governance,
            scope: wire.scope,
            session_relation: wire.session_relation,
            cutover: wire.cutover,
            evidence: wire.evidence,
            resource_continuity_profile_digest: wire.resource_continuity_profile_digest,
        };
        if value.schema_sha256 != *expected_schema_sha256 {
            return Err(InvalidResidentRecordV1::SchemaCoordinate {
                family: "resource continuity profile",
            });
        }
        value.verify()?;
        Ok(value)
    }
}

/// Reprove the complete semantic algebra after structural decoding.
fn validate_resource_continuity_profile(
    value: &ResourceContinuityProfilePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    value.access.validate()?;
    value.copies.validate()?;
    value.convergence.validate(&value.access)?;
    value.mechanisms.validate()?;
    value.scope.validate()
}

/// Enforce a bounded strict order by one stable key.
fn strictly_sorted_by_key<T, K: Ord>(
    relation: &'static str,
    values: &[T],
    key: impl Fn(&T) -> &K,
) -> Result<(), InvalidResidentRecordV1> {
    bounded_len(relation, values.len())?;
    if values.windows(2).all(|pair| key(&pair[0]) < key(&pair[1])) {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::NonCanonical { relation })
    }
}

/// Construct one contextual continuity-admission rejection.
fn continuity_reject(invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "resource continuity profile",
        invariant,
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/continuity.rs"]
mod tests;
