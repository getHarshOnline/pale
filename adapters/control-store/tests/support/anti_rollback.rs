//! In-process independent-anchor model for adapter integration tests.
//!
//! This model intentionally shares no storage with `SQLite`. It exercises the
//! port's exact compare-and-advance contract, but it is not a qualified
//! production backend and provides no cross-store atomic transaction.

use std::collections::BTreeMap;

use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorKey, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    PreparedAntiRollbackAnchorTransition,
};

#[derive(Default)]
pub(crate) struct RetainedAntiRollbackAnchors {
    anchors: BTreeMap<AntiRollbackAnchorKey, AntiRollbackAnchor>,
}

impl RetainedAntiRollbackAnchors {
    pub(crate) fn with_anchor(anchor: AntiRollbackAnchor) -> Self {
        Self {
            anchors: BTreeMap::from([(anchor.key(), anchor)]),
        }
    }
}

impl AntiRollbackAnchorPort for RetainedAntiRollbackAnchors {
    fn read(
        &mut self,
        key: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, AntiRollbackAnchorError> {
        Ok(self.anchors.get(key).cloned())
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        let observation = transition.classify_observed(self.anchors.get(transition.key()));
        match observation {
            AntiRollbackAnchorObservation::Missing if transition.expected().is_none() => {
                self.anchors
                    .insert(transition.key().clone(), transition.next().clone());
                Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
            }
            AntiRollbackAnchorObservation::ExactPredecessor => {
                self.anchors
                    .insert(transition.key().clone(), transition.next().clone());
                Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
            }
            AntiRollbackAnchorObservation::ExactSuccessor => {
                Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay)
            }
            observation => Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(
                observation,
            )),
        }
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        Ok(transition.classify_observed(self.anchors.get(transition.key())))
    }
}
