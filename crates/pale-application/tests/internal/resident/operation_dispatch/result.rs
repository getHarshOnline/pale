//! Test construction for retained provider-attempt preflight boundaries.

use super::*;

impl<'a> RecoveredProviderAttempt<'a> {
    pub(in crate::resident::operation_dispatch) fn from_retained_fixture(
        selection: AttemptSelection,
        parents: AttemptSelectionContext<'a>,
    ) -> Result<Self, OperationAttemptError> {
        selection.validate(&parents)?;
        Ok(Self { selection, parents })
    }
}
