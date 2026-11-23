//! Current recipient authority for retained workspace inspection.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use pale_core::records::{InteractionDisclosureCeilingV1, ResidentPrincipalIdV1};

use super::{
    AdmittedWorkspaceComparisonInspection, AdmittedWorkspaceInspection, PortableWorkspaceUniverse,
    WorkspaceInspectionAdmissionOwner, WorkspaceInspectionAdmissionRequest,
    WorkspaceInspectionAuthorityGeneration, WorkspaceInspectionBudget,
    WorkspaceInspectionBudgetLimits, WorkspaceInspectionDenied, WorkspaceInspectionRecipient,
    WorkspaceInspectionRecipientAdmissionRequest,
};

/// One principal's current maximum disclosure for workspace inspection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInspectionGrant {
    principal_id: ResidentPrincipalIdV1,
    disclosure_ceiling: InteractionDisclosureCeilingV1,
}

impl WorkspaceInspectionGrant {
    /// Define one recipient grant for the current authority generation.
    #[must_use]
    pub const fn new(
        principal_id: ResidentPrincipalIdV1,
        disclosure_ceiling: InteractionDisclosureCeilingV1,
    ) -> Self {
        Self {
            principal_id,
            disclosure_ceiling,
        }
    }

    /// Borrow the principal that may receive an inspection.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Borrow the maximum disclosure captured for that principal.
    #[must_use]
    pub const fn disclosure_ceiling(&self) -> &InteractionDisclosureCeilingV1 {
        &self.disclosure_ceiling
    }
}

/// Invalid replacement of current workspace-inspection authority.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WorkspaceInspectionAuthorityError {
    /// Two grants name the same principal.
    #[error("workspace inspection authority contains a duplicate principal")]
    DuplicatePrincipal,
    /// The monotone process-local generation can no longer advance.
    #[error("workspace inspection authority generation is exhausted")]
    GenerationExhausted,
}

struct WorkspaceInspectionAuthorityState {
    generation: WorkspaceInspectionAuthorityGeneration,
    generation_counter: u64,
    grants: BTreeMap<ResidentPrincipalIdV1, InteractionDisclosureCeilingV1>,
}

/// Current process-local authority for recipient-qualified workspace inspection.
///
/// The resident or application host replaces the complete grant set when
/// authority changes. Replacement advances one private monotone generation.
/// Every issued handle rechecks that generation and the recipient's exact
/// disclosure ceiling before it reveals a selector, cursor, or result.
pub struct WorkspaceInspectionAuthority {
    state: RwLock<WorkspaceInspectionAuthorityState>,
}

impl std::fmt::Debug for WorkspaceInspectionAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceInspectionAuthority")
            .finish_non_exhaustive()
    }
}

impl WorkspaceInspectionAuthority {
    /// Start authority at its first generation with one complete grant set.
    ///
    /// # Errors
    /// Returns an error when two grants name the same principal.
    pub fn new(
        grants: impl IntoIterator<Item = WorkspaceInspectionGrant>,
    ) -> Result<Self, WorkspaceInspectionAuthorityError> {
        Ok(Self {
            state: RwLock::new(WorkspaceInspectionAuthorityState {
                generation: WorkspaceInspectionAuthorityGeneration::from_counter(1),
                generation_counter: 1,
                grants: collect_grants(grants)?,
            }),
        })
    }

    /// Replace every recipient grant and invalidate all earlier handles.
    ///
    /// The replacement is atomic with its generation advance. An empty set
    /// revokes every recipient without weakening retained owner custody.
    ///
    /// # Errors
    /// Returns an error for duplicate principals or an exhausted generation.
    pub fn replace_grants(
        &self,
        grants: impl IntoIterator<Item = WorkspaceInspectionGrant>,
    ) -> Result<(), WorkspaceInspectionAuthorityError> {
        let grants = collect_grants(grants)?;
        let mut state = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let generation_counter = state
            .generation_counter
            .checked_add(1)
            .ok_or(WorkspaceInspectionAuthorityError::GenerationExhausted)?;
        state.generation_counter = generation_counter;
        state.generation = WorkspaceInspectionAuthorityGeneration::from_counter(generation_counter);
        state.grants = grants;
        Ok(())
    }

    /// Issue one recipient-qualified handle for an exact retained compilation.
    ///
    /// # Errors
    /// Returns a non-revealing denial when the principal has no current grant.
    pub fn issue_workspace(
        self: &Arc<Self>,
        owner: Arc<PortableWorkspaceUniverse>,
        principal_id: &ResidentPrincipalIdV1,
    ) -> Result<AdmittedWorkspaceInspection, WorkspaceInspectionDenied> {
        self.issue_workspace_with_budget(
            owner,
            principal_id,
            WorkspaceInspectionBudgetLimits::default(),
        )
    }

    /// Issue one handle with an explicit aggregate journey budget.
    ///
    /// # Errors
    /// Returns a non-revealing denial when the principal has no current grant.
    pub fn issue_workspace_with_budget(
        self: &Arc<Self>,
        owner: Arc<PortableWorkspaceUniverse>,
        principal_id: &ResidentPrincipalIdV1,
        limits: WorkspaceInspectionBudgetLimits,
    ) -> Result<AdmittedWorkspaceInspection, WorkspaceInspectionDenied> {
        let recipient = self.capture_recipient(principal_id)?;
        let admission = Arc::new(BoundWorkspaceInspectionAdmission::workspace(
            Arc::clone(self),
            &owner,
        ));
        Ok(AdmittedWorkspaceInspection::new_with_budget(
            owner,
            recipient,
            admission,
            Arc::new(WorkspaceInspectionBudget::new(limits)),
        ))
    }

    /// Issue both workspace handles and their exact ordered comparison together.
    ///
    /// All three handles share one cumulative budget and the same captured
    /// authority generation. Equal public digests cannot substitute either
    /// retained compiler owner.
    ///
    /// # Errors
    /// Returns a non-revealing denial when the principal has no current grant.
    pub fn issue_comparison(
        self: &Arc<Self>,
        prior: Arc<PortableWorkspaceUniverse>,
        current: Arc<PortableWorkspaceUniverse>,
        principal_id: &ResidentPrincipalIdV1,
    ) -> Result<WorkspaceInspectionPairIssuance, WorkspaceInspectionDenied> {
        self.issue_comparison_with_budget(
            prior,
            current,
            principal_id,
            WorkspaceInspectionBudgetLimits::default(),
        )
    }

    /// Issue an exact ordered comparison with one explicit shared budget.
    ///
    /// # Errors
    /// Returns a non-revealing denial when the principal has no current grant.
    pub fn issue_comparison_with_budget(
        self: &Arc<Self>,
        prior: Arc<PortableWorkspaceUniverse>,
        current: Arc<PortableWorkspaceUniverse>,
        principal_id: &ResidentPrincipalIdV1,
        limits: WorkspaceInspectionBudgetLimits,
    ) -> Result<WorkspaceInspectionPairIssuance, WorkspaceInspectionDenied> {
        let recipient = self.capture_recipient(principal_id)?;
        let budget = Arc::new(WorkspaceInspectionBudget::new(limits));
        let prior_admission = Arc::new(BoundWorkspaceInspectionAdmission::workspace(
            Arc::clone(self),
            &prior,
        ));
        let current_admission = Arc::new(BoundWorkspaceInspectionAdmission::workspace(
            Arc::clone(self),
            &current,
        ));
        let comparison_admission = Arc::new(BoundWorkspaceInspectionAdmission::comparison(
            Arc::clone(self),
            &prior,
            &current,
        ));
        let prior_inspection = AdmittedWorkspaceInspection::new_with_budget(
            Arc::clone(&prior),
            recipient.clone(),
            prior_admission,
            Arc::clone(&budget),
        );
        let current_inspection = AdmittedWorkspaceInspection::new_with_budget(
            Arc::clone(&current),
            recipient.clone(),
            current_admission,
            Arc::clone(&budget),
        );
        let comparison = AdmittedWorkspaceComparisonInspection::new_with_budget(
            prior,
            current,
            recipient,
            comparison_admission,
            budget,
        );
        Ok(WorkspaceInspectionPairIssuance {
            prior: prior_inspection,
            current: current_inspection,
            comparison,
        })
    }

    fn capture_recipient(
        &self,
        principal_id: &ResidentPrincipalIdV1,
    ) -> Result<WorkspaceInspectionRecipient, WorkspaceInspectionDenied> {
        let state = self
            .state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let disclosure_ceiling = state
            .grants
            .get(principal_id)
            .cloned()
            .ok_or(WorkspaceInspectionDenied)?;
        Ok(WorkspaceInspectionRecipient::new(
            principal_id.clone(),
            state.generation,
            disclosure_ceiling,
        ))
    }

    fn admits(&self, recipient: &WorkspaceInspectionRecipient) -> bool {
        let state = self
            .state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.generation == recipient.authority_generation()
            && state.grants.get(recipient.principal_id()) == Some(recipient.disclosure_ceiling())
    }
}

/// One jointly issued predecessor, current workspace, and comparison handle.
pub struct WorkspaceInspectionPairIssuance {
    prior: AdmittedWorkspaceInspection,
    current: AdmittedWorkspaceInspection,
    comparison: AdmittedWorkspaceComparisonInspection,
}

impl std::fmt::Debug for WorkspaceInspectionPairIssuance {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceInspectionPairIssuance")
            .finish_non_exhaustive()
    }
}

impl WorkspaceInspectionPairIssuance {
    /// Consume the joint issuance into its three opaque application handles.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        AdmittedWorkspaceInspection,
        AdmittedWorkspaceInspection,
        AdmittedWorkspaceComparisonInspection,
    ) {
        (self.prior, self.current, self.comparison)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkspaceInspectionOwnerCoordinate {
    workspace_id: String,
    principal_universe_digest: String,
    semantic_ir_digest: String,
    admission_evidence_digest: String,
}

impl WorkspaceInspectionOwnerCoordinate {
    fn capture(owner: &PortableWorkspaceUniverse) -> Self {
        Self {
            workspace_id: owner.workspace_id().to_owned(),
            principal_universe_digest: owner.principal_universe_digest().to_owned(),
            semantic_ir_digest: owner.semantic_ir_digest().to_owned(),
            admission_evidence_digest: owner.admission_evidence_digest().to_owned(),
        }
    }

    fn matches_recipient(
        &self,
        request: &WorkspaceInspectionRecipientAdmissionRequest<'_>,
    ) -> bool {
        request.workspace_id() == self.workspace_id.as_str()
            && request.principal_universe_digest() == self.principal_universe_digest.as_str()
            && request.semantic_ir_digest() == self.semantic_ir_digest.as_str()
            && request.admission_evidence_digest() == self.admission_evidence_digest.as_str()
    }

    fn matches_current(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool {
        request.workspace_id() == self.workspace_id.as_str()
            && request.principal_universe_digest() == self.principal_universe_digest.as_str()
            && request.semantic_ir_digest() == self.semantic_ir_digest.as_str()
            && request.admission_evidence_digest() == self.admission_evidence_digest.as_str()
    }

    fn matches_prior(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool {
        request.prior_workspace_id() == Some(self.workspace_id.as_str())
            && request.prior_principal_universe_digest()
                == Some(self.principal_universe_digest.as_str())
            && request.prior_semantic_ir_digest() == Some(self.semantic_ir_digest.as_str())
            && request.prior_admission_evidence_digest()
                == Some(self.admission_evidence_digest.as_str())
    }
}

struct BoundWorkspaceInspectionAdmission {
    authority: Arc<WorkspaceInspectionAuthority>,
    current: WorkspaceInspectionOwnerCoordinate,
    prior: Option<WorkspaceInspectionOwnerCoordinate>,
}

impl BoundWorkspaceInspectionAdmission {
    fn workspace(
        authority: Arc<WorkspaceInspectionAuthority>,
        current: &PortableWorkspaceUniverse,
    ) -> Self {
        Self {
            authority,
            current: WorkspaceInspectionOwnerCoordinate::capture(current),
            prior: None,
        }
    }

    fn comparison(
        authority: Arc<WorkspaceInspectionAuthority>,
        prior: &PortableWorkspaceUniverse,
        current: &PortableWorkspaceUniverse,
    ) -> Self {
        Self {
            authority,
            current: WorkspaceInspectionOwnerCoordinate::capture(current),
            prior: Some(WorkspaceInspectionOwnerCoordinate::capture(prior)),
        }
    }
}

impl WorkspaceInspectionAdmissionOwner for BoundWorkspaceInspectionAdmission {
    fn admits_recipient(&self, request: &WorkspaceInspectionRecipientAdmissionRequest<'_>) -> bool {
        self.current.matches_recipient(request) && self.authority.admits(request.recipient())
    }

    fn admits(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool {
        self.current.matches_current(request)
            && request.query().requires_comparison() == self.prior.is_some()
            && match &self.prior {
                Some(prior) => prior.matches_prior(request),
                None => {
                    request.prior_workspace_id().is_none()
                        && request.prior_principal_universe_digest().is_none()
                        && request.prior_semantic_ir_digest().is_none()
                        && request.prior_admission_evidence_digest().is_none()
                }
            }
            && self.authority.admits(request.recipient())
    }
}

fn collect_grants(
    grants: impl IntoIterator<Item = WorkspaceInspectionGrant>,
) -> Result<
    BTreeMap<ResidentPrincipalIdV1, InteractionDisclosureCeilingV1>,
    WorkspaceInspectionAuthorityError,
> {
    let mut collected = BTreeMap::new();
    for grant in grants {
        if collected
            .insert(grant.principal_id, grant.disclosure_ceiling)
            .is_some()
        {
            return Err(WorkspaceInspectionAuthorityError::DuplicatePrincipal);
        }
    }
    Ok(collected)
}
