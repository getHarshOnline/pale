//! Monotonic write-verification and post-compaction continuity epochs.

use std::num::NonZeroU64;

use pale_core::engine_wire::{RebaselineDebtV1, VerificationDebtV1, VerificationRequirementV1};

use super::{SessionRuntimeState, SessionStateError, StateEffect};

/// Strength of verification required for one outstanding write epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationRequirement {
    /// Ordinary structured writes receive one completion reminder.
    Standard,
    /// A protected configuration exception requires an exact repository verifier.
    ProtectedConfiguration,
}

impl VerificationRequirement {
    const fn strongest(self, other: Self) -> Self {
        match (self, other) {
            (Self::ProtectedConfiguration, _) | (_, Self::ProtectedConfiguration) => {
                Self::ProtectedConfiguration
            }
            (Self::Standard, Self::Standard) => Self::Standard,
        }
    }

    pub(super) const fn to_wire(self) -> VerificationRequirementV1 {
        match self {
            Self::Standard => VerificationRequirementV1::Standard,
            Self::ProtectedConfiguration => VerificationRequirementV1::ProtectedConfiguration,
        }
    }

    pub(super) const fn from_wire(value: VerificationRequirementV1) -> Self {
        match value {
            VerificationRequirementV1::Standard => Self::Standard,
            VerificationRequirementV1::ProtectedConfiguration => Self::ProtectedConfiguration,
        }
    }
}

/// One outstanding write epoch and its completion-gate strength.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationDebt {
    epoch: NonZeroU64,
    completion_prompted: bool,
    requirement: VerificationRequirement,
}

impl VerificationDebt {
    pub(super) const fn to_wire(&self) -> VerificationDebtV1 {
        VerificationDebtV1 {
            epoch: self.epoch,
            completion_prompted: self.completion_prompted,
            requirement: self.requirement.to_wire(),
        }
    }

    pub(super) const fn from_wire(wire: VerificationDebtV1) -> Self {
        Self {
            epoch: wire.epoch,
            completion_prompted: wire.completion_prompted,
            requirement: VerificationRequirement::from_wire(wire.requirement),
        }
    }

    /// Monotonic epoch of the write this debt represents.
    pub fn epoch(&self) -> u64 {
        self.epoch.get()
    }

    /// Whether completion guidance was already emitted for this epoch.
    pub fn completion_prompted(&self) -> bool {
        self.completion_prompted
    }

    /// Verification strength required to discharge this debt.
    pub fn requirement(&self) -> VerificationRequirement {
        self.requirement
    }

    /// Return whether verification evidence belongs to the current mutation epoch.
    pub(super) fn is_consistent_with(&self, epoch: u64) -> bool {
        self.epoch.get() == epoch
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Verification debt created by a rebaseline transition at one epoch.
pub(super) struct RebaselineDebt {
    epoch: NonZeroU64,
}

impl RebaselineDebt {
    pub(super) const fn to_wire(&self) -> RebaselineDebtV1 {
        RebaselineDebtV1 { epoch: self.epoch }
    }

    pub(super) const fn from_wire(wire: RebaselineDebtV1) -> Self {
        Self { epoch: wire.epoch }
    }

    /// Return whether rebaseline debt still belongs to the current epoch.
    pub(super) fn is_consistent_with(&self, epoch: u64) -> bool {
        self.epoch.get() == epoch
    }
}

impl SessionRuntimeState {
    /// Borrow the latest outstanding verification debt.
    pub fn verification_debt(&self) -> Option<&VerificationDebt> {
        self.verification_debt.as_ref()
    }

    /// Record a write and allocate a newer verification epoch.
    ///
    /// A protected requirement cannot be weakened by a later ordinary write.
    ///
    /// # Errors
    ///
    /// Leaves debt unchanged when no newer epoch can be represented.
    pub fn mark_verification_pending(
        &mut self,
        requirement: VerificationRequirement,
    ) -> Result<StateEffect, SessionStateError> {
        let epoch = self
            .verification_epoch
            .checked_add(1)
            .ok_or(SessionStateError::VerificationEpochOverflow)?;
        let debt_epoch =
            NonZeroU64::new(epoch).ok_or(SessionStateError::VerificationEpochOverflow)?;
        let requirement = self
            .verification_debt
            .as_ref()
            .map_or(requirement, |debt| debt.requirement.strongest(requirement));
        self.verification_epoch = epoch;
        self.verification_debt = Some(VerificationDebt {
            epoch: debt_epoch,
            completion_prompted: false,
            requirement,
        });
        Ok(StateEffect::VerificationPending { epoch, requirement })
    }

    /// Clear only the verification debt observed before a verifier ran.
    pub fn clear_verification_if_epoch(&mut self, expected: u64) -> Option<StateEffect> {
        let current = self.verification_debt.as_ref()?;
        if current.epoch.get() != expected {
            return None;
        }
        self.verification_debt = None;
        Some(StateEffect::VerificationCleared { epoch: expected })
    }

    /// Mark the one completion prompt for the observed verification epoch.
    pub fn mark_verification_prompted(&mut self, expected: u64) -> Option<StateEffect> {
        let current = self.verification_debt.as_mut()?;
        if current.epoch.get() != expected || current.completion_prompted {
            return None;
        }
        current.completion_prompted = true;
        Some(StateEffect::VerificationPrompted { epoch: expected })
    }

    /// Arm a fresh context refresh after compaction.
    ///
    /// # Errors
    ///
    /// Leaves debt unchanged when no newer epoch can be represented.
    pub fn mark_rebaseline_pending(&mut self) -> Result<StateEffect, SessionStateError> {
        let epoch = self
            .rebaseline_epoch
            .checked_add(1)
            .ok_or(SessionStateError::RebaselineEpochOverflow)?;
        let debt_epoch =
            NonZeroU64::new(epoch).ok_or(SessionStateError::RebaselineEpochOverflow)?;
        self.rebaseline_epoch = epoch;
        self.rebaseline_debt = Some(RebaselineDebt { epoch: debt_epoch });
        Ok(StateEffect::RebaselinePending { epoch })
    }

    /// Consume only the refresh epoch observed before rendering context.
    pub fn clear_rebaseline_if_epoch(&mut self, expected: u64) -> Option<StateEffect> {
        let current = self.rebaseline_debt.as_ref()?;
        if current.epoch.get() != expected {
            return None;
        }
        self.rebaseline_debt = None;
        Some(StateEffect::RebaselineCleared { epoch: expected })
    }

    /// Epoch of the context refresh currently awaiting delivery.
    pub fn pending_rebaseline_epoch(&self) -> Option<u64> {
        self.rebaseline_debt.as_ref().map(|debt| debt.epoch.get())
    }
}

#[cfg(test)]
#[path = "../../tests/internal/session_state/verification.rs"]
mod tests;
