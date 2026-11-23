//! Test-only access to retained provider recovery evidence for A2A sends.

use super::*;

pub(in crate::resident) use super::checkpoint::test_support::A2aSendRecoveryFixture;

impl<'operation> A2aProviderDispatch<'operation> {
    pub(in crate::resident) fn recovery_fixture(
        &self,
    ) -> Option<super::super::provider::test_support::RetainedProviderRecoveryFixture<'operation>>
    {
        match self {
            Self::Fresh(dispatch) => Some(dispatch.recovery_fixture()),
            Self::Recovered(_) => None,
        }
    }
}
