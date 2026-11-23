//! The small record that carries an agent session's mode, operation and recent
//! activity from one hook call to the next.
//!
//! `hooks` and `operation_admission` build on it, and it uses `operation_admission`
//! types in return.
//!
//! The serialized record is admitted as one invariant-bearing value. Domain
//! modules own its mode, operation, recency, and continuity transitions; none
//! perform I/O.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use pale_core::engine_wire::SessionRuntimeStateV1;
use pale_core::records::{
    ModeAxis, PackageResourceRefV1, RecordSchemaIdentity, SchemaDigest, SchemaId,
};
use serde::{Deserialize, Serialize};

mod effect;
mod modes;
mod operation;
mod recency;
mod verification;

#[cfg(test)]
#[path = "../../tests/internal/session_state.rs"]
mod tests;

pub use effect::{SessionStateError, StateEffect};
pub use operation::ActiveOperationLease;
pub use verification::{VerificationDebt, VerificationRequirement};

use operation::lease_within_bounds;
pub(crate) use operation::{InflightAdmissionStatus, InflightConsumption};
use verification::RebaselineDebt;

// One persisted session record owns all of these ceilings. Deserialization and
// every prospective transition validate the same bounded state shape.
/// Maximum independently active mode axes in one session.
const MAX_ACTIVE_MODES: usize = 8;
/// Maximum exact lexical paths retained for first-touch semantics.
const MAX_TOUCHED_PATHS: usize = 1_024;
/// Maximum recency-ordered capture identities retained in one session.
const MAX_CAPTURES: usize = 128;
/// Maximum bytes retained directly for one session observation.
const MAX_STATE_TEXT_BYTES: usize = 512;
/// Maximum native actions authorized by one active operation lease.
const MAX_AUTHORIZED_ACTIONS: usize = 128;
/// Maximum concurrent tool invocations tracked by one lease.
const MAX_INFLIGHT_TOOLS: usize = 128;

/// Bounded state shared by separate hook processes within one session.
///
/// Deserialization validates the complete record. Public transitions preserve
/// that invariant and return explicit effects; adapters own persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRuntimeState {
    schema: SchemaId,
    schema_sha256: SchemaDigest,
    active_modes: BTreeMap<ModeAxis, PackageResourceRefV1>,
    touched: BTreeSet<String>,
    captures: VecDeque<String>,
    active_operation: Option<ActiveOperationLease>,
    operation_epoch: u64,
    verification_epoch: u64,
    verification_debt: Option<VerificationDebt>,
    rebaseline_epoch: u64,
    rebaseline_debt: Option<RebaselineDebt>,
}

impl Serialize for SessionRuntimeState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SessionRuntimeStateV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            active_modes: self.active_modes.clone(),
            touched: self.touched.clone(),
            captures: self.captures.clone(),
            active_operation: self
                .active_operation
                .as_ref()
                .map(ActiveOperationLease::to_wire),
            operation_epoch: self.operation_epoch,
            verification_epoch: self.verification_epoch,
            verification_debt: self
                .verification_debt
                .as_ref()
                .map(VerificationDebt::to_wire),
            rebaseline_epoch: self.rebaseline_epoch,
            rebaseline_debt: self.rebaseline_debt.as_ref().map(RebaselineDebt::to_wire),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SessionRuntimeState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = SessionRuntimeStateV1::deserialize(deserializer)?;
        let state = Self {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            active_modes: wire.active_modes,
            touched: wire.touched,
            captures: wire.captures,
            active_operation: wire
                .active_operation
                .map(ActiveOperationLease::try_from_wire)
                .transpose()
                .map_err(serde::de::Error::custom)?,
            operation_epoch: wire.operation_epoch,
            verification_epoch: wire.verification_epoch,
            verification_debt: wire.verification_debt.map(VerificationDebt::from_wire),
            rebaseline_epoch: wire.rebaseline_epoch,
            rebaseline_debt: wire.rebaseline_debt.map(RebaselineDebt::from_wire),
        };
        state.validate_loaded().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

impl Default for SessionRuntimeState {
    fn default() -> Self {
        let identity = pale_contracts::record_schema_identity("session-runtime-state.v1")
            .expect("embedded session-runtime-state descriptor must be valid");
        Self {
            schema: identity.schema,
            schema_sha256: identity.schema_sha256,
            active_modes: BTreeMap::new(),
            touched: BTreeSet::new(),
            captures: VecDeque::new(),
            active_operation: None,
            operation_epoch: 0,
            verification_epoch: 0,
            verification_debt: None,
            rebaseline_epoch: 0,
            rebaseline_debt: None,
        }
    }
}

impl SessionRuntimeState {
    /// Exact standalone payload identity selected by the embedded descriptor registry.
    ///
    /// # Errors
    ///
    /// Returns [`pale_contracts::SchemaError`] when the compiled schema registry
    /// is incoherent or lacks the session-runtime-state contract.
    pub fn record_schema_identity() -> Result<RecordSchemaIdentity, pale_contracts::SchemaError> {
        pale_contracts::record_schema_identity("session-runtime-state.v1")
    }

    fn validate_loaded(&self) -> Result<(), SessionStateError> {
        pale_contracts::validate_record(self, "session-runtime-state.v1")
            .map_err(|_| SessionStateError::SchemaIdentityInvalid)?;
        Self::validate_mode_selection(&self.active_modes)?;
        if self.touched.len() > MAX_TOUCHED_PATHS
            || self
                .touched
                .iter()
                .any(|value| value.len() > MAX_STATE_TEXT_BYTES)
        {
            return Err(SessionStateError::FirstTouchBound);
        }
        if self.captures.len() > MAX_CAPTURES
            || self
                .captures
                .iter()
                .any(|value| value.len() > MAX_STATE_TEXT_BYTES)
            || self.captures.iter().collect::<BTreeSet<_>>().len() != self.captures.len()
        {
            return Err(SessionStateError::CapturesBound);
        }
        if self
            .active_operation
            .as_ref()
            .is_some_and(|lease| !lease_within_bounds(lease))
        {
            return Err(SessionStateError::OperationLeaseBound);
        }
        if self
            .active_operation
            .as_ref()
            .is_some_and(|lease| lease.activation_epoch() != self.operation_epoch)
        {
            return Err(SessionStateError::OperationEpochInvalid);
        }
        if self
            .verification_debt
            .as_ref()
            .is_some_and(|debt| !debt.is_consistent_with(self.verification_epoch))
        {
            return Err(SessionStateError::VerificationStateInvalid);
        }
        if self
            .rebaseline_debt
            .as_ref()
            .is_some_and(|debt| !debt.is_consistent_with(self.rebaseline_epoch))
        {
            return Err(SessionStateError::RebaselineStateInvalid);
        }
        Ok(())
    }
}
