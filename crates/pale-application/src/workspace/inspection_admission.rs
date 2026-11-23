//! Recipient-qualified portable workspace inspection.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pale_core::records::{
    InteractionDisclosureCeilingV1, LogicalScopeRefV1, OrganizationalNodeRefV1,
    ResidentPrincipalIdV1, WorkspaceDerivationSubjectV1,
};

use super::{
    OrganizationInspectionCursor, OrganizationInspectionError, OrganizationInspectionPage,
    PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectInspectionCursor,
    PortableWorkspaceSubjectInspectionError, PortableWorkspaceUniverse,
};

/// Monotone generation of the authority used to admit an inspection recipient.
///
/// This is application state, not a wire coordinate. A retained inspection
/// fails closed as soon as its authority owner reports another generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkspaceInspectionAuthorityGeneration(u64);

impl WorkspaceInspectionAuthorityGeneration {
    /// Capture the generation reported by the authority owner.
    #[must_use]
    pub(crate) const fn from_counter(counter: u64) -> Self {
        Self(counter)
    }
}

/// Recipient, authority generation and maximum disclosure for one inspection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInspectionRecipient {
    principal_id: ResidentPrincipalIdV1,
    authority_generation: WorkspaceInspectionAuthorityGeneration,
    disclosure_ceiling: InteractionDisclosureCeilingV1,
}

/// Conservative aggregate ceilings for one retained inspection journey.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceInspectionBudgetLimits {
    maximum_work_units: u64,
    maximum_copied_bytes: u64,
    maximum_encoded_bytes: u64,
    maximum_elapsed: Duration,
}

impl WorkspaceInspectionBudgetLimits {
    /// Create explicit work, copy, encoding and elapsed-time ceilings.
    #[must_use]
    pub const fn new(
        maximum_work_units: u64,
        maximum_copied_bytes: u64,
        maximum_encoded_bytes: u64,
        maximum_elapsed: Duration,
    ) -> Self {
        Self {
            maximum_work_units,
            maximum_copied_bytes,
            maximum_encoded_bytes,
            maximum_elapsed,
        }
    }
}

impl Default for WorkspaceInspectionBudgetLimits {
    fn default() -> Self {
        Self::new(
            1_048_576,
            16 * 1024 * 1024,
            32 * 1024 * 1024,
            Duration::from_secs(30),
        )
    }
}

#[derive(Debug)]
struct WorkspaceInspectionBudgetState {
    work_units: u64,
    copied_bytes: u64,
    encoded_bytes: u64,
}

/// Shared budget custody for nested queries and continuations.
///
/// The budget charges Pale-owned work and output. It does not pretend to
/// measure the retained native graph behind an `Arc`; callers must limit the
/// number of retained compilation handles separately.
#[derive(Debug)]
pub struct WorkspaceInspectionBudget {
    limits: WorkspaceInspectionBudgetLimits,
    started: Instant,
    state: Mutex<WorkspaceInspectionBudgetState>,
}

impl WorkspaceInspectionBudget {
    /// Start one aggregate budget at the current monotonic instant.
    #[must_use]
    pub(crate) fn new(limits: WorkspaceInspectionBudgetLimits) -> Self {
        Self {
            limits,
            started: Instant::now(),
            state: Mutex::new(WorkspaceInspectionBudgetState {
                work_units: 0,
                copied_bytes: 0,
                encoded_bytes: 0,
            }),
        }
    }

    fn charge(
        &self,
        work_units: u64,
        copied_bytes: u64,
        encoded_bytes: u64,
    ) -> Result<(), WorkspaceInspectionBudgetExceeded> {
        if self.started.elapsed() > self.limits.maximum_elapsed {
            return Err(WorkspaceInspectionBudgetExceeded::Deadline);
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.work_units = state
            .work_units
            .checked_add(work_units)
            .ok_or(WorkspaceInspectionBudgetExceeded::Work)?;
        state.copied_bytes = state
            .copied_bytes
            .checked_add(copied_bytes)
            .ok_or(WorkspaceInspectionBudgetExceeded::CopiedBytes)?;
        state.encoded_bytes = state
            .encoded_bytes
            .checked_add(encoded_bytes)
            .ok_or(WorkspaceInspectionBudgetExceeded::EncodedBytes)?;
        if state.work_units > self.limits.maximum_work_units {
            return Err(WorkspaceInspectionBudgetExceeded::Work);
        }
        if state.copied_bytes > self.limits.maximum_copied_bytes {
            return Err(WorkspaceInspectionBudgetExceeded::CopiedBytes);
        }
        if state.encoded_bytes > self.limits.maximum_encoded_bytes {
            return Err(WorkspaceInspectionBudgetExceeded::EncodedBytes);
        }
        Ok(())
    }

    /// Charge copied and encoded output after a native call returns.
    ///
    /// A failed charge does not refund work already performed.
    ///
    /// # Errors
    /// Returns the first aggregate ceiling exceeded by this charge.
    pub fn charge_output(
        &self,
        copied_bytes: u64,
        encoded_bytes: u64,
    ) -> Result<(), WorkspaceInspectionBudgetExceeded> {
        self.charge(0, copied_bytes, encoded_bytes)
    }
}

/// Exact aggregate ceiling that stopped an inspection journey.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WorkspaceInspectionBudgetExceeded {
    /// Native/query work-unit ceiling was exceeded.
    #[error("workspace inspection work budget is exhausted")]
    Work,
    /// Copied Pale-owned output ceiling was exceeded.
    #[error("workspace inspection copied-byte budget is exhausted")]
    CopiedBytes,
    /// Encoded delivery ceiling was exceeded.
    #[error("workspace inspection encoded-byte budget is exhausted")]
    EncodedBytes,
    /// Monotonic deadline expired before the next bounded operation.
    #[error("workspace inspection deadline has expired")]
    Deadline,
}

impl WorkspaceInspectionRecipient {
    /// Bind one recipient to authority state selected outside the compiler.
    #[must_use]
    pub(crate) const fn new(
        principal_id: ResidentPrincipalIdV1,
        authority_generation: WorkspaceInspectionAuthorityGeneration,
        disclosure_ceiling: InteractionDisclosureCeilingV1,
    ) -> Self {
        Self {
            principal_id,
            authority_generation,
            disclosure_ceiling,
        }
    }

    /// Exact recipient principal.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Authority generation captured for this retained inspection.
    #[must_use]
    pub const fn authority_generation(&self) -> WorkspaceInspectionAuthorityGeneration {
        self.authority_generation
    }

    /// Maximum disclosure selected by the authority owner.
    #[must_use]
    pub const fn disclosure_ceiling(&self) -> &InteractionDisclosureCeilingV1 {
        &self.disclosure_ceiling
    }
}

/// Exact typed operation presented to the application authority owner.
///
/// Callers never submit this value to execute a query. The application creates
/// it from a typed method and opaque cursor custody before asking for a decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceInspectionQuery {
    /// Read the identity-and-count summary of one retained workspace.
    WorkspaceSummary,
    /// Associate a retained observation with this exact workspace and profile.
    ProfileObservationAssociation,
    /// Page all selected organization records.
    Organization,
    /// Find an exact local organization endpoint.
    OrganizationEndpoint,
    /// Page relations incident to an exact endpoint.
    OrganizationIncidentRelations,
    /// Page services incident to an exact scope.
    OrganizationIncidentServices,
    /// Read named source evidence for one exact retained organization record.
    OrganizationSources,
    /// Read one bounded detail collection from an exact organization record.
    OrganizationDetail,
    /// Inspect one compiler subject and its source evidence.
    Subject,
    /// Read one page of direct proof dependencies for an exact subject.
    DirectDependencies,
    /// Read identity summaries for one exact retained comparison pair.
    ComparisonSummary,
    /// Enumerate exact subjects across a retained comparison pair.
    ComparisonSubjects,
    /// Project one page of exact source-change roots.
    ComparisonChangeRoots,
    /// Project one page of the exact current impact partition.
    ComparisonImpacts,
    /// Compare one exact subject in a retained pair.
    ComparisonSubject,
    /// Read contributing change roots for one exact comparison subject.
    ComparisonContributingRoots,
    /// Read direct proof dependencies from one exact side of a retained comparison.
    ComparisonDirectDependencies,
    /// Inspect one subject on one exact side of a retained comparison.
    ComparisonSubjectInspection,
    /// Find an exact organization record on the other comparison side.
    OrganizationCorrespondence,
}

impl WorkspaceInspectionQuery {
    /// Whether this operation requires an exact predecessor owner.
    #[must_use]
    pub(crate) const fn requires_comparison(self) -> bool {
        matches!(
            self,
            Self::ComparisonSummary
                | Self::ComparisonSubjects
                | Self::ComparisonChangeRoots
                | Self::ComparisonImpacts
                | Self::ComparisonSubject
                | Self::ComparisonContributingRoots
                | Self::ComparisonDirectDependencies
                | Self::ComparisonSubjectInspection
                | Self::OrganizationCorrespondence
        )
    }
}

/// Organization detail collection named by an admitted SDK method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceInspectionOrganizationSection {
    /// Authority-axis resources.
    AxisResources,
    /// Leadership participants.
    LeadershipParticipants,
    /// Leadership scopes.
    LeadershipScopes,
    /// Communication local resolutions.
    CommunicationResolutions,
    /// One communication closure.
    CommunicationClosure,
    /// Lens standards.
    LensStandards,
    /// Lens conditions.
    LensConditions,
    /// Lens rights.
    LensRights,
    /// Service assignments.
    ServiceAssignments,
    /// One assignment's stewards.
    AssignmentStewards,
    /// One assignment's pods.
    AssignmentPods,
    /// Service rights.
    ServiceRights,
    /// Service controllers.
    ServiceControllers,
    /// Service authorities.
    ServiceAuthorities,
    /// Native references.
    NativeReferences,
    /// Inspection coordinate classes.
    InspectionCoordinates,
    /// Inspection fact states.
    InspectionStates,
    /// Responsibility selections.
    ResponsibilitySelections,
    /// Responsibility residences.
    ResponsibilityResidences,
    /// Responsibility projections.
    ResponsibilityProjections,
    /// One projection's native references.
    ProjectionReferences,
    /// Responsibility genealogy links.
    ResponsibilityGenealogy,
    /// One genealogy link's responsibilities.
    GenealogyResponsibilities,
    /// One genealogy link's projections.
    GenealogyProjections,
    /// One genealogy link's occurrences.
    GenealogyOccurrences,
}

/// Exact side selected within an admitted comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceInspectionComparisonSide {
    /// The predecessor compilation.
    Prior,
    /// The current compilation.
    Current,
}

/// Read-only admission request for one exact retained compiler owner.
pub(crate) struct WorkspaceInspectionAdmissionRequest<'a> {
    recipient: &'a WorkspaceInspectionRecipient,
    workspace_id: &'a str,
    principal_universe_digest: &'a str,
    semantic_ir_digest: &'a str,
    admission_evidence_digest: &'a str,
    prior_workspace_id: Option<&'a str>,
    prior_principal_universe_digest: Option<&'a str>,
    prior_semantic_ir_digest: Option<&'a str>,
    prior_admission_evidence_digest: Option<&'a str>,
    query: WorkspaceInspectionQuery,
}

/// Read-only recipient preflight before selector or continuation validation.
pub(crate) struct WorkspaceInspectionRecipientAdmissionRequest<'a> {
    recipient: &'a WorkspaceInspectionRecipient,
    workspace_id: &'a str,
    principal_universe_digest: &'a str,
    semantic_ir_digest: &'a str,
    admission_evidence_digest: &'a str,
}

impl<'a> WorkspaceInspectionRecipientAdmissionRequest<'a> {
    /// Recipient and captured authority state.
    #[must_use]
    pub(crate) const fn recipient(&self) -> &'a WorkspaceInspectionRecipient {
        self.recipient
    }
    /// Workspace identity reported by the retained compiler owner.
    #[must_use]
    pub(crate) const fn workspace_id(&self) -> &'a str {
        self.workspace_id
    }
    /// Principal-relative universe identity reported by that owner.
    #[must_use]
    pub(crate) const fn principal_universe_digest(&self) -> &'a str {
        self.principal_universe_digest
    }
    /// Semantic identity reported by that owner.
    #[must_use]
    pub(crate) const fn semantic_ir_digest(&self) -> &'a str {
        self.semantic_ir_digest
    }
    /// Admission-evidence identity reported by that owner.
    #[must_use]
    pub(crate) const fn admission_evidence_digest(&self) -> &'a str {
        self.admission_evidence_digest
    }
}

impl<'a> WorkspaceInspectionAdmissionRequest<'a> {
    /// Recipient and captured authority state.
    #[must_use]
    pub(crate) const fn recipient(&self) -> &'a WorkspaceInspectionRecipient {
        self.recipient
    }
    /// Workspace identity reported by the retained compiler owner.
    #[must_use]
    pub(crate) const fn workspace_id(&self) -> &'a str {
        self.workspace_id
    }
    /// Principal-relative universe identity reported by that owner.
    #[must_use]
    pub(crate) const fn principal_universe_digest(&self) -> &'a str {
        self.principal_universe_digest
    }
    /// Semantic identity reported by that owner.
    #[must_use]
    pub(crate) const fn semantic_ir_digest(&self) -> &'a str {
        self.semantic_ir_digest
    }
    /// Admission-evidence identity reported by that owner.
    #[must_use]
    pub(crate) const fn admission_evidence_digest(&self) -> &'a str {
        self.admission_evidence_digest
    }
    /// Prior workspace identity for a comparison query.
    #[must_use]
    pub(crate) const fn prior_workspace_id(&self) -> Option<&'a str> {
        self.prior_workspace_id
    }
    /// Prior principal-universe identity for a comparison query.
    #[must_use]
    pub(crate) const fn prior_principal_universe_digest(&self) -> Option<&'a str> {
        self.prior_principal_universe_digest
    }
    /// Prior semantic identity for a comparison query.
    #[must_use]
    pub(crate) const fn prior_semantic_ir_digest(&self) -> Option<&'a str> {
        self.prior_semantic_ir_digest
    }
    /// Prior admission-evidence identity for a comparison query.
    #[must_use]
    pub(crate) const fn prior_admission_evidence_digest(&self) -> Option<&'a str> {
        self.prior_admission_evidence_digest
    }
    /// Typed operation created by the application for this call.
    #[must_use]
    pub(crate) const fn query(&self) -> WorkspaceInspectionQuery {
        self.query
    }
}

/// Internal decision boundary for recipient-qualified inspection.
pub(crate) trait WorkspaceInspectionAdmissionOwner: Send + Sync + 'static {
    /// Atomically decide whether this recipient may learn selector or cursor validity.
    ///
    /// The decision must compare [`WorkspaceInspectionRecipient::authority_generation`]
    /// with the same owner-native snapshot that supplies the admission decision.
    fn admits_recipient(&self, request: &WorkspaceInspectionRecipientAdmissionRequest<'_>) -> bool;

    /// Atomically decide whether this exact recipient, owner and typed operation may run.
    ///
    /// The decision must compare [`WorkspaceInspectionRecipient::authority_generation`]
    /// with the same owner-native snapshot that supplies the admission decision.
    fn admits(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool;
}

/// Admission failed before compiler enumeration or lookup began.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("workspace inspection is not admitted for this recipient and authority generation")]
pub struct WorkspaceInspectionDenied;

/// Recipient admission or aggregate budget refusal before publication.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceInspectionAccessError {
    /// Recipient disclosure or authority generation was refused.
    #[error(transparent)]
    Denied(#[from] WorkspaceInspectionDenied),
    /// One aggregate inspection ceiling was exhausted.
    #[error(transparent)]
    Budget(#[from] WorkspaceInspectionBudgetExceeded),
}

/// Issued authorization for one exact organization-source call.
pub struct AdmittedOrganizationSources {
    record: super::OrganizationRecordHandle,
    continuation: bool,
    maximum_inspected: u16,
    maximum_returned: u16,
}

impl AdmittedOrganizationSources {
    /// Verify that the SDK is executing the exact admitted source call.
    #[must_use]
    pub fn matches(
        &self,
        record: &super::OrganizationRecordHandle,
        continuation: bool,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> bool {
        self.record.same_record(record)
            && self.continuation == continuation
            && self.maximum_inspected == maximum_inspected
            && self.maximum_returned == maximum_returned
    }
}

/// Issued authorization for one exact organization-detail call.
pub struct AdmittedOrganizationDetail {
    record: super::OrganizationRecordHandle,
    section: WorkspaceInspectionOrganizationSection,
    continuation: bool,
    maximum_inspected: u16,
    maximum_returned: u16,
}

impl AdmittedOrganizationDetail {
    /// Verify that the SDK is executing the exact admitted detail call.
    #[must_use]
    pub fn matches(
        &self,
        record: &super::OrganizationRecordHandle,
        section: WorkspaceInspectionOrganizationSection,
        continuation: bool,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> bool {
        self.record.same_record(record)
            && self.section == section
            && self.continuation == continuation
            && self.maximum_inspected == maximum_inspected
            && self.maximum_returned == maximum_returned
    }
}

macro_rules! comparison_permit {
    ($name:ident { $($field:ident : $ty:ty),+ $(,)? }) => {
        /// Issued authorization for one exact comparison query.
        pub struct $name { $(pub(crate) $field: $ty),+ }
    };
}

comparison_permit!(AdmittedComparisonSubjects {
    continuation: bool,
    limit: u16
});
comparison_permit!(AdmittedComparisonChangeRoots {
    offset: u32,
    limit: u16
});
comparison_permit!(AdmittedComparisonImpacts {
    offset: u32,
    limit: u16
});
comparison_permit!(AdmittedComparisonSubject {
    subject: WorkspaceDerivationSubjectV1
});
comparison_permit!(AdmittedDirectDependencies {
    subject: WorkspaceDerivationSubjectV1,
    continuation: bool,
    limit: u16,
});
comparison_permit!(AdmittedComparisonContributingRoots {
    subject: WorkspaceDerivationSubjectV1,
    continuation: bool,
    limit: u16,
});
comparison_permit!(AdmittedComparisonDirectDependencies {
    selected_side: WorkspaceInspectionComparisonSide,
    subject: WorkspaceDerivationSubjectV1,
    continuation: bool,
    limit: u16,
});
comparison_permit!(AdmittedComparisonSubjectInspection {
    selected_side: WorkspaceInspectionComparisonSide,
    subject: WorkspaceDerivationSubjectV1,
    continuation: bool,
    input_limit: u16,
    maximum_inspected: usize,
    maximum_returned: usize,
});

impl AdmittedComparisonSubjects {
    /// Verify the admitted continuation state and raw limit.
    #[must_use]
    pub const fn matches(&self, continuation: bool, limit: u16) -> bool {
        self.continuation == continuation && self.limit == limit
    }
}
impl AdmittedComparisonChangeRoots {
    /// Verify the admitted page offset and raw limit.
    #[must_use]
    pub const fn matches(&self, offset: u32, limit: u16) -> bool {
        self.offset == offset && self.limit == limit
    }
}
impl AdmittedComparisonImpacts {
    /// Verify the admitted page offset and raw limit.
    #[must_use]
    pub const fn matches(&self, offset: u32, limit: u16) -> bool {
        self.offset == offset && self.limit == limit
    }
}
impl AdmittedComparisonSubject {
    /// Verify the exact admitted compiler subject.
    #[must_use]
    pub fn matches(&self, subject: &WorkspaceDerivationSubjectV1) -> bool {
        &self.subject == subject
    }
}
impl AdmittedDirectDependencies {
    /// Verify the exact subject, continuation state and raw limit.
    #[must_use]
    pub fn matches(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        continuation: bool,
        limit: u16,
    ) -> bool {
        &self.subject == subject && self.continuation == continuation && self.limit == limit
    }
}
impl AdmittedComparisonContributingRoots {
    /// Verify the exact subject, continuation state and raw limit.
    #[must_use]
    pub fn matches(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        continuation: bool,
        limit: u16,
    ) -> bool {
        &self.subject == subject && self.continuation == continuation && self.limit == limit
    }
}
impl AdmittedComparisonDirectDependencies {
    /// Verify the side, subject, continuation state and raw page limit.
    #[must_use]
    pub fn matches(
        &self,
        selected_side: WorkspaceInspectionComparisonSide,
        subject: &WorkspaceDerivationSubjectV1,
        continuation: bool,
        limit: u16,
    ) -> bool {
        self.selected_side == selected_side
            && &self.subject == subject
            && self.continuation == continuation
            && self.limit == limit
    }
}
impl AdmittedComparisonSubjectInspection {
    /// Verify the side, subject, continuation state and all inspection limits.
    #[must_use]
    pub fn matches(
        &self,
        selected_side: WorkspaceInspectionComparisonSide,
        subject: &WorkspaceDerivationSubjectV1,
        continuation: bool,
        input_limit: u16,
        maximum_inspected: usize,
        maximum_returned: usize,
    ) -> bool {
        self.selected_side == selected_side
            && &self.subject == subject
            && self.continuation == continuation
            && self.input_limit == input_limit
            && self.maximum_inspected == maximum_inspected
            && self.maximum_returned == maximum_returned
    }
}

/// Issued authorization for one exact cross-generation organization lookup.
pub struct AdmittedOrganizationCorrespondence {
    selected_side: WorkspaceInspectionComparisonSide,
    record: super::OrganizationRecordHandle,
    continuation: bool,
    maximum_inspected: u16,
}

impl AdmittedOrganizationCorrespondence {
    /// Verify that the SDK is executing the exact admitted correspondence call.
    #[must_use]
    pub fn matches(
        &self,
        selected_side: WorkspaceInspectionComparisonSide,
        record: &super::OrganizationRecordHandle,
        continuation: bool,
        maximum_inspected: u16,
    ) -> bool {
        self.selected_side == selected_side
            && self.record.same_record(record)
            && self.continuation == continuation
            && self.maximum_inspected == maximum_inspected
    }
}

/// Exact retained compiler owner plus current recipient authority.
///
/// This type does not expose the compiler. Its typed methods perform admission
/// and the matching compiler primitive as one operation. Every continuation is
/// re-admitted and remains bound to the exact retained owner.
#[derive(Clone)]
pub struct AdmittedWorkspaceInspection {
    owner: Arc<PortableWorkspaceUniverse>,
    recipient: WorkspaceInspectionRecipient,
    admission_owner: Arc<dyn WorkspaceInspectionAdmissionOwner>,
    budget: Arc<WorkspaceInspectionBudget>,
}

impl std::fmt::Debug for AdmittedWorkspaceInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedWorkspaceInspection")
            .field("recipient", &self.recipient)
            .finish_non_exhaustive()
    }
}

impl AdmittedWorkspaceInspection {
    /// Retain one compiler input under an explicitly shared aggregate budget.
    #[must_use]
    pub(crate) fn new_with_budget(
        owner: Arc<PortableWorkspaceUniverse>,
        recipient: WorkspaceInspectionRecipient,
        admission_owner: Arc<dyn WorkspaceInspectionAdmissionOwner>,
        budget: Arc<WorkspaceInspectionBudget>,
    ) -> Self {
        Self {
            owner,
            recipient,
            admission_owner,
            budget,
        }
    }

    /// Recipient bound to this admitted inspection.
    #[must_use]
    pub const fn recipient(&self) -> &WorkspaceInspectionRecipient {
        &self.recipient
    }

    /// Test exact retained ownership. Equal public identities are insufficient.
    #[must_use]
    pub fn belongs_to(&self, owner: &Arc<PortableWorkspaceUniverse>) -> bool {
        Arc::ptr_eq(&self.owner, owner)
    }

    fn admit(
        &self,
        query: WorkspaceInspectionQuery,
        prior: Option<&PortableWorkspaceUniverse>,
    ) -> Result<(), WorkspaceInspectionDenied> {
        let request = WorkspaceInspectionAdmissionRequest {
            recipient: &self.recipient,
            workspace_id: self.owner.workspace_id(),
            principal_universe_digest: self.owner.principal_universe_digest(),
            semantic_ir_digest: self.owner.semantic_ir_digest(),
            admission_evidence_digest: self.owner.admission_evidence_digest(),
            prior_workspace_id: prior.map(PortableWorkspaceUniverse::workspace_id),
            prior_principal_universe_digest: prior
                .map(PortableWorkspaceUniverse::principal_universe_digest),
            prior_semantic_ir_digest: prior.map(PortableWorkspaceUniverse::semantic_ir_digest),
            prior_admission_evidence_digest: prior
                .map(PortableWorkspaceUniverse::admission_evidence_digest),
            query,
        };
        self.admission_owner
            .admits(&request)
            .then_some(())
            .ok_or(WorkspaceInspectionDenied)
    }

    /// Check recipient authority before validating any selector or continuation.
    ///
    /// # Errors
    /// Returns a non-revealing denial when recipient authority is stale or absent.
    pub fn preflight(&self) -> Result<(), WorkspaceInspectionDenied> {
        self.admission_owner
            .admits_recipient(&WorkspaceInspectionRecipientAdmissionRequest {
                recipient: &self.recipient,
                workspace_id: self.owner.workspace_id(),
                principal_universe_digest: self.owner.principal_universe_digest(),
                semantic_ir_digest: self.owner.semantic_ir_digest(),
                admission_evidence_digest: self.owner.admission_evidence_digest(),
            })
            .then_some(())
            .ok_or(WorkspaceInspectionDenied)
    }

    /// Admit the identity-and-count summary of this retained workspace.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before summary projection.
    pub fn authorize_summary(&self) -> Result<(), WorkspaceInspectionAccessError> {
        self.prepare(WorkspaceInspectionQuery::WorkspaceSummary, None, 1)
    }

    /// Admit association of a retained observation with this workspace.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before profile, source or
    /// observation coordinates are compared.
    pub fn authorize_profile_observation_association(
        &self,
    ) -> Result<(), WorkspaceInspectionAccessError> {
        self.prepare(
            WorkspaceInspectionQuery::ProfileObservationAssociation,
            None,
            1,
        )
    }

    fn prepare(
        &self,
        query: WorkspaceInspectionQuery,
        prior: Option<&PortableWorkspaceUniverse>,
        work_units: u64,
    ) -> Result<(), WorkspaceInspectionAccessError> {
        self.preflight()?;
        self.admit(query, prior)?;
        self.budget.charge(work_units, 0, 0)?;
        Ok(())
    }

    /// Shared output budget for eligible SDK and Scene projections.
    #[must_use]
    pub fn budget(&self) -> &Arc<WorkspaceInspectionBudget> {
        &self.budget
    }

    /// Page all selected organization records after recipient admission.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before the compiler query starts.
    pub fn inspect_organization(
        &self,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<
        Result<OrganizationInspectionPage<'_>, OrganizationInspectionError>,
        WorkspaceInspectionAccessError,
    > {
        self.prepare(
            WorkspaceInspectionQuery::Organization,
            None,
            u64::from(maximum_inspected),
        )?;
        Ok(self
            .owner
            .inspect_organization(after, maximum_inspected, maximum_returned))
    }

    /// Find one exact organization endpoint after recipient admission.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before the compiler query starts.
    pub fn inspect_organization_endpoint(
        &self,
        endpoint: &OrganizationalNodeRefV1,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<
        Result<OrganizationInspectionPage<'_>, OrganizationInspectionError>,
        WorkspaceInspectionAccessError,
    > {
        self.prepare(
            WorkspaceInspectionQuery::OrganizationEndpoint,
            None,
            u64::from(maximum_inspected),
        )?;
        Ok(self.owner.inspect_organization_endpoint(
            endpoint,
            after,
            maximum_inspected,
            maximum_returned,
        ))
    }

    /// Page relations incident to one endpoint after recipient admission.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before the compiler query starts.
    pub fn inspect_organization_incident_relations(
        &self,
        endpoint: &OrganizationalNodeRefV1,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<
        Result<OrganizationInspectionPage<'_>, OrganizationInspectionError>,
        WorkspaceInspectionAccessError,
    > {
        self.prepare(
            WorkspaceInspectionQuery::OrganizationIncidentRelations,
            None,
            u64::from(maximum_inspected),
        )?;
        Ok(self.owner.inspect_organization_incident_relations(
            endpoint,
            after,
            maximum_inspected,
            maximum_returned,
        ))
    }

    /// Page services incident to one scope after recipient admission.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before the compiler query starts.
    pub fn inspect_organization_incident_services(
        &self,
        scope: &LogicalScopeRefV1,
        after: Option<&OrganizationInspectionCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<
        Result<OrganizationInspectionPage<'_>, OrganizationInspectionError>,
        WorkspaceInspectionAccessError,
    > {
        self.prepare(
            WorkspaceInspectionQuery::OrganizationIncidentServices,
            None,
            u64::from(maximum_inspected),
        )?;
        Ok(self.owner.inspect_organization_incident_services(
            scope,
            after,
            maximum_inspected,
            maximum_returned,
        ))
    }

    /// Admit one exact record-source query.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before source inspection starts.
    pub fn authorize_organization_sources(
        &self,
        record: &super::OrganizationRecordHandle,
        continuation: bool,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<AdmittedOrganizationSources, WorkspaceInspectionAccessError> {
        self.prepare(
            WorkspaceInspectionQuery::OrganizationSources,
            None,
            u64::from(maximum_inspected),
        )?;
        Ok(AdmittedOrganizationSources {
            record: record.clone(),
            continuation,
            maximum_inspected,
            maximum_returned,
        })
    }

    /// Admit one exact detail collection query.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before detail inspection starts.
    pub fn authorize_organization_detail(
        &self,
        record: &super::OrganizationRecordHandle,
        section: WorkspaceInspectionOrganizationSection,
        continuation: bool,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<AdmittedOrganizationDetail, WorkspaceInspectionAccessError> {
        self.prepare(
            WorkspaceInspectionQuery::OrganizationDetail,
            None,
            u64::from(maximum_inspected),
        )?;
        Ok(AdmittedOrganizationDetail {
            record: record.clone(),
            section,
            continuation,
            maximum_inspected,
            maximum_returned,
        })
    }

    /// Admit one page of direct proof dependencies.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before dependency inspection starts.
    pub fn authorize_direct_dependencies(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        continuation: bool,
        limit: u16,
    ) -> Result<AdmittedDirectDependencies, WorkspaceInspectionAccessError> {
        self.prepare(
            WorkspaceInspectionQuery::DirectDependencies,
            None,
            u64::from(limit),
        )?;
        Ok(AdmittedDirectDependencies {
            subject: subject.clone(),
            continuation,
            limit,
        })
    }

    /// Inspect one subject and its source evidence after recipient admission.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before subject inspection starts.
    pub fn inspect_subject(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        after: Option<&PortableWorkspaceSubjectInspectionCursor>,
        input_limit: u16,
        maximum_inspected: usize,
        maximum_returned: usize,
    ) -> Result<
        Result<PortableWorkspaceSubjectInspection<'_>, PortableWorkspaceSubjectInspectionError>,
        WorkspaceInspectionAccessError,
    > {
        self.prepare(
            WorkspaceInspectionQuery::Subject,
            None,
            u64::from(input_limit)
                .saturating_add(u64::try_from(maximum_inspected).unwrap_or(u64::MAX))
                .saturating_add(u64::try_from(maximum_returned).unwrap_or(u64::MAX)),
        )?;
        Ok(self.owner.inspect_subject(
            subject,
            after,
            input_limit,
            maximum_inspected,
            maximum_returned,
        ))
    }
}

/// Exact retained comparison pair under one recipient authority owner.
///
/// Pale SDK owns comparison derivation. This application envelope binds every
/// projected comparison call to both exact immutable compiler owners before the
/// SDK can enumerate or copy a result.
#[derive(Clone)]
pub struct AdmittedWorkspaceComparisonInspection {
    prior: Arc<PortableWorkspaceUniverse>,
    current: AdmittedWorkspaceInspection,
}

impl std::fmt::Debug for AdmittedWorkspaceComparisonInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedWorkspaceComparisonInspection")
            .field("recipient", self.current.recipient())
            .finish_non_exhaustive()
    }
}

impl AdmittedWorkspaceComparisonInspection {
    /// Shared output budget for this exact comparison journey.
    #[must_use]
    pub fn budget(&self) -> &Arc<WorkspaceInspectionBudget> {
        self.current.budget()
    }

    /// Retain one exact pair under an explicitly shared aggregate budget.
    #[must_use]
    pub(crate) fn new_with_budget(
        prior: Arc<PortableWorkspaceUniverse>,
        current: Arc<PortableWorkspaceUniverse>,
        recipient: WorkspaceInspectionRecipient,
        admission_owner: Arc<dyn WorkspaceInspectionAdmissionOwner>,
        budget: Arc<WorkspaceInspectionBudget>,
    ) -> Self {
        Self {
            prior,
            current: AdmittedWorkspaceInspection::new_with_budget(
                current,
                recipient,
                admission_owner,
                budget,
            ),
        }
    }

    /// Test exact ordered pair custody. Equal public identities are insufficient.
    #[must_use]
    pub fn belongs_to(
        &self,
        prior: &Arc<PortableWorkspaceUniverse>,
        current: &Arc<PortableWorkspaceUniverse>,
    ) -> bool {
        Arc::ptr_eq(&self.prior, prior) && self.current.belongs_to(current)
    }

    /// Admit the complete identity summaries of this exact ordered pair.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before summary projection.
    pub fn authorize_summary(&self) -> Result<(), WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::ComparisonSummary,
            Some(&self.prior),
            1,
        )
    }

    /// Admit exact comparison-subject enumeration.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before subject enumeration starts.
    pub fn authorize_subjects(
        &self,
        continuation: bool,
        limit: u16,
    ) -> Result<AdmittedComparisonSubjects, WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::ComparisonSubjects,
            Some(&self.prior),
            u64::from(limit),
        )?;
        Ok(AdmittedComparisonSubjects {
            continuation,
            limit,
        })
    }

    /// Run exact change-root projection after pair admission.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before projection starts.
    pub fn authorize_change_roots(
        &self,
        offset: u32,
        limit: u16,
    ) -> Result<AdmittedComparisonChangeRoots, WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::ComparisonChangeRoots,
            Some(&self.prior),
            u64::from(limit),
        )?;
        Ok(AdmittedComparisonChangeRoots { offset, limit })
    }

    /// Run exact impact projection after pair admission.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before projection starts.
    pub fn authorize_impacts(
        &self,
        offset: u32,
        limit: u16,
    ) -> Result<AdmittedComparisonImpacts, WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::ComparisonImpacts,
            Some(&self.prior),
            u64::from(limit),
        )?;
        Ok(AdmittedComparisonImpacts { offset, limit })
    }

    /// Compare one typed subject after pair admission.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before comparison starts.
    pub fn authorize_compare_subject(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> Result<AdmittedComparisonSubject, WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::ComparisonSubject,
            Some(&self.prior),
            2,
        )?;
        Ok(AdmittedComparisonSubject {
            subject: subject.clone(),
        })
    }

    /// Admit one contributing-root page for an exact subject.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before root inspection starts.
    pub fn authorize_contributing_roots(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        continuation: bool,
        limit: u16,
    ) -> Result<AdmittedComparisonContributingRoots, WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::ComparisonContributingRoots,
            Some(&self.prior),
            u64::from(limit),
        )?;
        Ok(AdmittedComparisonContributingRoots {
            subject: subject.clone(),
            continuation,
            limit,
        })
    }

    /// Admit one direct-dependency page on an exact comparison side.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before dependency inspection starts.
    pub fn authorize_direct_dependencies(
        &self,
        selected_side: WorkspaceInspectionComparisonSide,
        subject: &WorkspaceDerivationSubjectV1,
        continuation: bool,
        limit: u16,
    ) -> Result<AdmittedComparisonDirectDependencies, WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::ComparisonDirectDependencies,
            Some(&self.prior),
            u64::from(limit),
        )?;
        Ok(AdmittedComparisonDirectDependencies {
            selected_side,
            subject: subject.clone(),
            continuation,
            limit,
        })
    }

    /// Admit one source-and-proof page on an exact comparison side.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before subject inspection starts.
    pub fn authorize_subject_inspection(
        &self,
        selected_side: WorkspaceInspectionComparisonSide,
        subject: &WorkspaceDerivationSubjectV1,
        continuation: bool,
        input_limit: u16,
        maximum_inspected: usize,
        maximum_returned: usize,
    ) -> Result<AdmittedComparisonSubjectInspection, WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::ComparisonSubjectInspection,
            Some(&self.prior),
            u64::from(input_limit)
                .saturating_add(u64::try_from(maximum_inspected).unwrap_or(u64::MAX))
                .saturating_add(u64::try_from(maximum_returned).unwrap_or(u64::MAX)),
        )?;
        Ok(AdmittedComparisonSubjectInspection {
            selected_side,
            subject: subject.clone(),
            continuation,
            input_limit,
            maximum_inspected,
            maximum_returned,
        })
    }

    /// Admit one cross-generation organization correspondence query.
    ///
    /// # Errors
    /// Returns a denial or aggregate-budget stop before correspondence starts.
    pub fn authorize_organization_correspondence(
        &self,
        selected_side: WorkspaceInspectionComparisonSide,
        record: &super::OrganizationRecordHandle,
        continuation: bool,
        maximum_inspected: u16,
    ) -> Result<AdmittedOrganizationCorrespondence, WorkspaceInspectionAccessError> {
        self.current.prepare(
            WorkspaceInspectionQuery::OrganizationCorrespondence,
            Some(&self.prior),
            u64::from(maximum_inspected),
        )?;
        Ok(AdmittedOrganizationCorrespondence {
            selected_side,
            record: record.clone(),
            continuation,
            maximum_inspected,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/inspection_admission.rs"]
mod tests;
