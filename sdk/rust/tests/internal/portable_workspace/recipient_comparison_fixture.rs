//! Test-only construction of application-issued comparison custody.

use super::*;

impl RecipientQualifiedWorkspaceComparisonInspection {
    pub(crate) fn from_issued(
        prior: PortableWorkspaceCompilation,
        current: PortableWorkspaceCompilation,
        comparison: PortableWorkspaceComparison,
        issued: pale_application::workspace::AdmittedWorkspaceComparisonInspection,
    ) -> Result<Self, RecipientQualifiedWorkspaceComparisonOwnerMismatch> {
        if !comparison.compares(&prior, &current)
            || !issued.belongs_to(&prior.universe, &current.universe)
        {
            return Err(RecipientQualifiedWorkspaceComparisonOwnerMismatch);
        }
        Ok(Self {
            prior,
            current,
            comparison,
            admitted: issued,
        })
    }
}
