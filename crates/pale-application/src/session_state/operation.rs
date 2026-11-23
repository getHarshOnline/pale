//! Turn-bounded operation leases and optimistic replacement.

use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;

use pale_core::engine_wire::{
    ActiveOperationLeaseV1, InflightAdmissionStatusV1, InflightToolAdmissionV1, OperationFailureV1,
};
use pale_core::records::{
    CapabilityRequirementIdV1, FailurePosture, OperationBindingDigest, OperationCatalogDigest,
    OperationContractDigest, PackageResourceRefV1, ToolAuthorizationDigest, TurnIdentityDigest,
};

use crate::operation_admission::{
    OperationActivation, ToolInvocationDigest, ToolUseIdentityDigest,
};

use super::{
    SessionRuntimeState, SessionStateError, MAX_AUTHORIZED_ACTIONS, MAX_INFLIGHT_TOOLS,
    MAX_STATE_TEXT_BYTES,
};

#[derive(Clone, PartialEq, Eq)]
struct InflightToolAdmission {
    invocation_digest: ToolInvocationDigest,
    requirement_id: Option<CapabilityRequirementIdV1>,
    status: InflightAdmissionStatus,
}

#[derive(Clone, Copy, PartialEq, Eq)]
/// Admission state of one correlated tool invocation.
pub(crate) enum InflightAdmissionStatus {
    /// Native confirmation is waiting for durable decision evidence.
    EvidencePending,
    /// The invocation may be correlated with one exact post-tool event.
    Admitted,
}

/// Exact outcome of consuming one correlated tool invocation.
pub(crate) enum InflightConsumption {
    Missing,
    Mismatch,
    NotAdmitted,
    Matched {
        requirement_id: Option<CapabilityRequirementIdV1>,
    },
}

/// One operation admission bound to a turn and exact sealed contracts.
#[derive(Clone, PartialEq, Eq)]
pub struct ActiveOperationLease {
    activation_epoch: NonZeroU64,
    operation_id: PackageResourceRefV1,
    contract_digest: OperationContractDigest,
    catalog_digest: OperationCatalogDigest,
    binding_digest: OperationBindingDigest,
    turn_digest: Option<TurnIdentityDigest>,
    authorized_actions: BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>,
    inflight_tools: BTreeMap<ToolUseIdentityDigest, InflightToolAdmission>,
    failure: Option<OperationFailure>,
}

impl fmt::Debug for ActiveOperationLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ActiveOperationLease")
            .field("activation_epoch", &self.activation_epoch)
            .field("operation_id", &self.operation_id)
            .field("contract_digest", &self.contract_digest)
            .field("catalog_digest", &self.catalog_digest)
            .field("binding_digest", &self.binding_digest)
            .field("turn_bound", &self.turn_digest.is_some())
            .field("authorized_action_count", &self.authorized_actions.len())
            .field("inflight_tool_count", &self.inflight_tools.len())
            .field("failure_present", &self.failure.is_some())
            .finish()
    }
}

impl ActiveOperationLease {
    fn from_activation(activation: OperationActivation, activation_epoch: NonZeroU64) -> Self {
        Self {
            activation_epoch,
            operation_id: activation.operation_id,
            contract_digest: activation.contract_digest,
            catalog_digest: activation.catalog_digest,
            binding_digest: activation.binding_digest,
            turn_digest: activation.turn_digest,
            authorized_actions: activation.authorized_actions,
            inflight_tools: BTreeMap::new(),
            failure: None,
        }
    }

    pub(super) fn to_wire(&self) -> ActiveOperationLeaseV1 {
        ActiveOperationLeaseV1 {
            activation_epoch: self.activation_epoch,
            operation_id: self.operation_id.clone(),
            contract_digest: self.contract_digest.clone(),
            catalog_digest: self.catalog_digest.clone(),
            binding_digest: self.binding_digest.clone(),
            turn_digest: self.turn_digest.clone(),
            authorized_actions: self.authorized_actions.clone(),
            inflight_tools: self
                .inflight_tools
                .iter()
                .map(|(identity, admission)| {
                    (
                        identity.as_artifact_digest().clone(),
                        InflightToolAdmissionV1 {
                            invocation_digest: admission
                                .invocation_digest
                                .as_artifact_digest()
                                .clone(),
                            requirement_id: admission.requirement_id.clone(),
                            status: match admission.status {
                                InflightAdmissionStatus::EvidencePending => {
                                    InflightAdmissionStatusV1::EvidencePending
                                }
                                InflightAdmissionStatus::Admitted => {
                                    InflightAdmissionStatusV1::Admitted
                                }
                            },
                        },
                    )
                })
                .collect(),
            failure: self.failure.as_ref().map(|failure| OperationFailureV1 {
                requirement_id: failure.requirement_id.clone(),
                posture: failure.posture,
            }),
        }
    }

    pub(super) fn try_from_wire(wire: ActiveOperationLeaseV1) -> Result<Self, SessionStateError> {
        let lease = Self {
            activation_epoch: wire.activation_epoch,
            operation_id: wire.operation_id,
            contract_digest: wire.contract_digest,
            catalog_digest: wire.catalog_digest,
            binding_digest: wire.binding_digest,
            turn_digest: wire.turn_digest,
            authorized_actions: wire.authorized_actions,
            inflight_tools: wire
                .inflight_tools
                .into_iter()
                .map(|(identity, admission)| {
                    (
                        ToolUseIdentityDigest::from_artifact_digest(identity),
                        InflightToolAdmission {
                            invocation_digest: ToolInvocationDigest::from_artifact_digest(
                                admission.invocation_digest,
                            ),
                            requirement_id: admission.requirement_id,
                            status: match admission.status {
                                InflightAdmissionStatusV1::EvidencePending => {
                                    InflightAdmissionStatus::EvidencePending
                                }
                                InflightAdmissionStatusV1::Admitted => {
                                    InflightAdmissionStatus::Admitted
                                }
                            },
                        },
                    )
                })
                .collect(),
            failure: wire.failure.map(|failure| OperationFailure {
                requirement_id: failure.requirement_id,
                posture: failure.posture,
            }),
        };
        if lease_within_bounds(&lease) {
            Ok(lease)
        } else {
            Err(SessionStateError::OperationLeaseBound)
        }
    }

    /// Epoch of the operation activation that minted this lease.
    pub(super) fn activation_epoch(&self) -> u64 {
        self.activation_epoch.get()
    }

    /// Selected operation identity.
    pub fn operation_id(&self) -> &PackageResourceRefV1 {
        &self.operation_id
    }

    /// Exact operation-contract digest admitted for this turn.
    pub fn contract_digest(&self) -> &OperationContractDigest {
        &self.contract_digest
    }

    /// Exact shared catalog digest containing the admitted contract.
    pub fn catalog_digest(&self) -> &OperationCatalogDigest {
        &self.catalog_digest
    }

    /// Exact route-binding digest through which the catalog was reached.
    pub fn binding_digest(&self) -> &OperationBindingDigest {
        &self.binding_digest
    }

    /// Host turn identity digest that confines this lease, when supplied.
    pub fn turn_digest(&self) -> Option<&TurnIdentityDigest> {
        self.turn_digest.as_ref()
    }

    /// Requirement IDs to exact owner-authorized request digests.
    pub(crate) fn authorized_actions(
        &self,
    ) -> &BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest> {
        &self.authorized_actions
    }

    /// Register one invocation identity once within this lease.
    pub(crate) fn register_inflight(
        &mut self,
        tool_use_id: ToolUseIdentityDigest,
        invocation_digest: ToolInvocationDigest,
        requirement_id: Option<&CapabilityRequirementIdV1>,
        status: InflightAdmissionStatus,
    ) -> Result<bool, SessionStateError> {
        if self.inflight_tools.len() >= MAX_INFLIGHT_TOOLS
            || requirement_id.is_some_and(|value| value.as_str().len() > MAX_STATE_TEXT_BYTES)
        {
            return Err(SessionStateError::OperationLeaseBound);
        }
        if self.inflight_tools.contains_key(&tool_use_id) {
            return Ok(false);
        }
        self.inflight_tools.insert(
            tool_use_id,
            InflightToolAdmission {
                invocation_digest,
                requirement_id: requirement_id.cloned(),
                status,
            },
        );
        Ok(true)
    }

    /// Promote only the matching evidence-pending invocation to admitted state.
    pub(crate) fn confirm_inflight(
        &mut self,
        tool_use_id: &ToolUseIdentityDigest,
        invocation_digest: &ToolInvocationDigest,
    ) -> bool {
        let Some(admission) = self.inflight_tools.get_mut(tool_use_id) else {
            return false;
        };
        if &admission.invocation_digest != invocation_digest
            || admission.status != InflightAdmissionStatus::EvidencePending
        {
            return false;
        }
        admission.status = InflightAdmissionStatus::Admitted;
        true
    }

    /// Consume and remove exactly one matching admitted invocation.
    pub(crate) fn consume_inflight(
        &mut self,
        tool_use_id: &ToolUseIdentityDigest,
        invocation_digest: &ToolInvocationDigest,
    ) -> InflightConsumption {
        let Some(admission) = self.inflight_tools.get(tool_use_id) else {
            return InflightConsumption::Missing;
        };
        if &admission.invocation_digest != invocation_digest {
            return InflightConsumption::Mismatch;
        }
        if admission.status != InflightAdmissionStatus::Admitted {
            return InflightConsumption::NotAdmitted;
        }
        let Some(admission) = self.inflight_tools.remove(tool_use_id) else {
            return InflightConsumption::Missing;
        };
        InflightConsumption::Matched {
            requirement_id: admission.requirement_id,
        }
    }

    /// Borrow the failure posture already attached to this lease.
    pub(crate) fn failure(&self) -> Option<&OperationFailure> {
        self.failure.as_ref()
    }

    /// Record the first failure or replace a deliberately non-sticky fail-open result.
    /// A retry posture admits one further failure, which becomes an escalation;
    /// all other failures remain attached until the lease itself is replaced.
    pub(crate) fn record_failure(
        &mut self,
        requirement_id: CapabilityRequirementIdV1,
        posture: FailurePosture,
    ) -> Result<(), SessionStateError> {
        if requirement_id.as_str().len() > MAX_STATE_TEXT_BYTES {
            return Err(SessionStateError::OperationLeaseBound);
        }
        let next_posture = match self.failure.as_ref() {
            None => posture,
            Some(failure) if failure.posture() == FailurePosture::FailOpen => posture,
            Some(failure)
                if failure.posture() == FailurePosture::Retry
                    && failure.requirement_id() == &requirement_id
                    && posture == FailurePosture::Retry =>
            {
                FailurePosture::Escalate
            }
            Some(_) => return Err(SessionStateError::OperationFailureSticky),
        };
        self.failure = Some(OperationFailure {
            requirement_id,
            posture: next_posture,
        });
        Ok(())
    }

    /// Clear retry posture only after the same failed requirement succeeds.
    pub(crate) fn clear_retry_failure(
        &mut self,
        requirement_id: &CapabilityRequirementIdV1,
    ) -> bool {
        let clears = self.failure.as_ref().is_some_and(|failure| {
            failure.posture() == FailurePosture::Retry && failure.requirement_id() == requirement_id
        });
        if clears {
            self.failure = None;
        }
        clears
    }
}

/// Failure state copied from a failed capability under the sealed contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationFailure {
    requirement_id: CapabilityRequirementIdV1,
    posture: FailurePosture,
}

impl OperationFailure {
    /// Capability requirement that produced the retained failure.
    pub(crate) fn requirement_id(&self) -> &CapabilityRequirementIdV1 {
        &self.requirement_id
    }

    /// Failure posture compiled for the retained requirement.
    pub(crate) fn posture(&self) -> FailurePosture {
        self.posture
    }
}

/// Check structural lease bounds without asserting that it is current or authorized.
pub(super) fn lease_within_bounds(lease: &ActiveOperationLease) -> bool {
    lease.authorized_actions().len() <= MAX_AUTHORIZED_ACTIONS
        && lease.inflight_tools.len() <= MAX_INFLIGHT_TOOLS
        && lease.operation_id().package_id.as_str().len()
            + lease.operation_id().resource_id.as_str().len()
            <= MAX_STATE_TEXT_BYTES
        && lease
            .authorized_actions()
            .keys()
            .all(|key| key.as_str().len() <= MAX_STATE_TEXT_BYTES)
        && lease.inflight_tools.values().all(|admission| {
            admission
                .requirement_id
                .as_ref()
                .is_none_or(|requirement| requirement.as_str().len() <= MAX_STATE_TEXT_BYTES)
        })
        && lease
            .failure()
            .is_none_or(|failure| failure.requirement_id().as_str().len() <= MAX_STATE_TEXT_BYTES)
}

impl SessionRuntimeState {
    /// Borrow the current turn-bounded operation lease.
    pub fn active_operation(&self) -> Option<&ActiveOperationLease> {
        self.active_operation.as_ref()
    }

    /// Replace a lease only while the persisted predecessor matches the caller's snapshot.
    ///
    /// # Errors
    ///
    /// Rejects an invalid or oversized replacement without changing state.
    pub fn replace_operation_if_current(
        &mut self,
        expected: Option<&ActiveOperationLease>,
        replacement: Option<&ActiveOperationLease>,
    ) -> Result<bool, SessionStateError> {
        if replacement.is_some_and(|lease| !lease_within_bounds(lease)) {
            return Err(SessionStateError::OperationLeaseBound);
        }
        if self.active_operation.as_ref() != expected {
            return Ok(false);
        }
        if let Some(next) = replacement {
            let same_activation = next.activation_epoch() == self.operation_epoch;
            let next_activation = self
                .operation_epoch
                .checked_add(1)
                .is_some_and(|epoch| next.activation_epoch() == epoch);
            if same_activation {
                let Some(current) = self.active_operation.as_ref() else {
                    return Err(SessionStateError::OperationEpochInvalid);
                };
                if !current.same_activation_identity(next) {
                    return Err(SessionStateError::OperationEpochInvalid);
                }
            } else if next_activation && next.failure().is_none() {
                self.operation_epoch = next.activation_epoch();
            } else {
                return Err(SessionStateError::OperationEpochInvalid);
            }
        }
        self.active_operation = replacement.cloned();
        Ok(true)
    }

    /// Bind a verified activation to the next monotonic session epoch.
    ///
    /// # Errors
    ///
    /// Leaves the prior lease intact when no newer epoch can be represented or
    /// the admitted activation exceeds a persisted-state bound.
    pub fn activate_operation(
        &mut self,
        activation: OperationActivation,
    ) -> Result<ActiveOperationLease, SessionStateError> {
        let epoch = self
            .operation_epoch
            .checked_add(1)
            .ok_or(SessionStateError::OperationEpochOverflow)?;
        let epoch = NonZeroU64::new(epoch).ok_or(SessionStateError::OperationEpochOverflow)?;
        let lease = ActiveOperationLease::from_activation(activation, epoch);
        if !lease_within_bounds(&lease) {
            return Err(SessionStateError::OperationLeaseBound);
        }
        self.operation_epoch = epoch.get();
        self.active_operation = Some(lease.clone());
        Ok(lease)
    }

    /// Remove and return the lease from this prospective snapshot.
    pub fn take_active_operation(&mut self) -> Option<ActiveOperationLease> {
        self.active_operation.take()
    }
}

impl ActiveOperationLease {
    fn same_activation_identity(&self, other: &Self) -> bool {
        self.activation_epoch == other.activation_epoch
            && self.operation_id == other.operation_id
            && self.contract_digest == other.contract_digest
            && self.catalog_digest == other.catalog_digest
            && self.binding_digest == other.binding_digest
            && self.turn_digest == other.turn_digest
            && self.authorized_actions == other.authorized_actions
    }
}
