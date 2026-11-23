//! Commit a competing database change after a real predecessor-anchor read.

use super::IndependentAnchor;
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorKey, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    PreparedAntiRollbackAnchorTransition,
};

pub(super) struct AfterPredecessor<'a, F> {
    anchor: &'a mut IndependentAnchor,
    commit: Option<F>,
}

impl<'a, F> AfterPredecessor<'a, F> {
    pub(super) fn new(anchor: &'a mut IndependentAnchor, commit: F) -> Self {
        Self {
            anchor,
            commit: Some(commit),
        }
    }

    pub(super) fn assert_used(&self) {
        assert!(
            self.commit.is_none(),
            "competing commit must run exactly once"
        );
    }
}

impl<F: FnOnce(&mut IndependentAnchor)> AntiRollbackAnchorPort for AfterPredecessor<'_, F> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.anchor.read(key)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        let observed = self.anchor.observe(transition)?;
        assert_eq!(observed, AntiRollbackAnchorObservation::ExactPredecessor);
        self.commit
            .take()
            .expect("one real predecessor read before native comparison")(self.anchor);
        Ok(observed)
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("the losing comparison must not advance the anchor")
    }
}
