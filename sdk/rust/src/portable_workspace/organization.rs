//! Owned organization observations over one retained complete compilation.

use std::sync::Arc;

use pale_application::workspace::{
    OrganizationInspectionCursor as ApplicationCursor,
    OrganizationInspectionError as ApplicationError, OrganizationRecordHandle as ApplicationRecord,
    OrganizationSourceCursor as ApplicationSourceCursor, OrganizationSources,
    PortableWorkspaceUniverse,
};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    RecordSchemaIdentity,
};

use super::PortableWorkspaceCompilation;
use crate::{error::FailureSpecification, ApiError, AuthoringOccurrence};

mod correspondence;
mod details;
mod node_queries;
pub use correspondence::{
    OrganizationComparisonSide, OrganizationCorrespondence, OrganizationCorrespondenceCursor,
    OrganizationCorrespondenceState, RecipientQualifiedOrganizationCorrespondence,
    RecipientQualifiedOrganizationCorrespondenceCursor,
    RecipientQualifiedOrganizationCorrespondenceState,
};
pub use node_queries::OrganizationEndpointLookup;
mod projection;
#[cfg(test)]
#[path = "../../tests/internal/portable_workspace/organization.rs"]
mod tests;
pub use details::{
    OrganizationCommunicationResolution, OrganizationCommunicationResolutionHandle,
    OrganizationDetailCursor, OrganizationDetailPage, OrganizationDetailParent,
    OrganizationDetailRows, OrganizationGenealogyHandle, OrganizationGenealogyLink,
    OrganizationInspectionCoordinate, OrganizationLensRight, OrganizationResidence,
    OrganizationResidenceClass, OrganizationResponsibilityProjection,
    OrganizationResponsibilityProjectionHandle, OrganizationServiceAssignment,
    OrganizationServiceAssignmentHandle, OrganizationStructuralKind,
    OrganizationStructuralSelection,
};
pub use projection::{
    OrganizationAttribution, OrganizationAxes, OrganizationConsumerAcceptanceAnswer,
    OrganizationDerivedClass, OrganizationDerivedSource, OrganizationDisclosure,
    OrganizationEffectCeiling, OrganizationEvidenceRequirement, OrganizationFactState,
    OrganizationGroupRef, OrganizationInterval, OrganizationNativeReadBackAnswer, OrganizationNode,
    OrganizationPlacementAnswer, OrganizationProjectionClass, OrganizationProviderAcceptanceAnswer,
    OrganizationProviderCompletionAnswer, OrganizationRef, OrganizationRelationClass,
    OrganizationRelationLocality, OrganizationResourceAnswer, OrganizationResponsibility,
    OrganizationResponsibilityLifecycle, OrganizationResponsibilityPlacement, OrganizationRoleRef,
    OrganizationRow, OrganizationScopeRef, OrganizationServiceRef, OrganizationSettlementAnswer,
    OrganizationTreeRef,
};

/// SDK query ceiling for copied UTF-8 string payload, not retained graph size.
///
/// This independently selected 4 MiB output-copy budget is not a consequence
/// of source admission and is not coupled to future source-file limits.
pub const MAX_ORGANIZATION_COPY_BYTES: u32 = 4 * 1024 * 1024;

/// Exact complete owner and application position, with every raw page limit.
///
/// No public constructor or serialization can recreate this custody. Clones
/// retain both semantic records and source maps, not merely a digest. This may
/// pin the complete compilation until the last independent handle is dropped.
#[derive(Clone)]
pub struct OrganizationCursor {
    owner: Arc<PortableWorkspaceUniverse>,
    application: ApplicationCursor,
    maximum_inspected: u16,
    maximum_returned: u16,
    maximum_copied_bytes: u32,
}

impl std::fmt::Debug for OrganizationCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationCursor").finish_non_exhaustive()
    }
}

/// Issued only from an actual application record, retaining complete source custody.
///
/// The shallow row is a copied observation. This handle, not that copy's fields,
/// selects subsequent detail/source queries. It grants no recipient disclosure,
/// resident authority, editor freshness or native effect permission.
#[derive(Clone)]
pub struct OrganizationRecordHandle {
    owner: Arc<PortableWorkspaceUniverse>,
    application: ApplicationRecord,
    diagnostic_identity: Arc<RecordSchemaIdentity>,
}

impl std::fmt::Debug for OrganizationRecordHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationRecordHandle")
            .finish_non_exhaustive()
    }
}

impl OrganizationRecordHandle {
    /// Whether two handles select one record under the same complete source owner.
    ///
    /// This supports exact joins between independently paged entries. It is not
    /// semantic equality or cross-generation correspondence. Equal values or
    /// digests from another compilation remain distinct source custody.
    #[must_use]
    pub fn same_record(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.application.same_record(&other.application)
    }

    /// Exact compilation membership, not a comparison of semantic digests.
    #[must_use]
    pub fn belongs_to(&self, compilation: &PortableWorkspaceCompilation) -> bool {
        Arc::ptr_eq(&self.owner, &compilation.universe)
    }

    /// Independently retained immutable workspace coordinate.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.owner.workspace_id()
    }

    /// Query the existing exact record/source owner without recompilation.
    ///
    /// Raw limits are 1..=256. The additional copy budget limits UTF-8 payload
    /// of returned labels and path fields before using the existing occurrence
    /// projection. It excludes vector capacity, numeric path steps, opaque
    /// cursor metadata and the retained compilation. No source text is copied.
    /// An empty named page may continue; unmapped means no exact mapping, not
    /// absence of source. Changing record or raw limits rejects continuation.
    ///
    /// # Errors
    /// Invalid limits/custody precede copying. Copy-budget exhaustion publishes
    /// no result or advanced cursor; the caller still owns its original cursor.
    pub fn sources(
        &self,
        after: Option<&OrganizationSourceCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<OrganizationSourcePage, ApiError> {
        let execute = || -> Result<OrganizationSourcePage, OrganizationFailure> {
            validate_limits(maximum_inspected, maximum_returned, maximum_copied_bytes)?;
            if after.is_some_and(|cursor| {
                !Arc::ptr_eq(&self.owner, &cursor.record.owner)
                    || cursor.maximum_inspected != maximum_inspected
                    || cursor.maximum_returned != maximum_returned
                    || cursor.maximum_copied_bytes != maximum_copied_bytes
            }) {
                return Err(OrganizationFailure::ContinuationMismatch);
            }
            let entry = self.owner.resolve_organization_record(&self.application)?;
            let source = entry.sources(
                after.map(|cursor| &cursor.application),
                maximum_inspected,
                maximum_returned,
            )?;
            let mut budget = CopyBudget::new(maximum_copied_bytes);
            let mapping = match source {
                OrganizationSources::UnmappedRecord => OrganizationSourceMapping::UnmappedRecord,
                OrganizationSources::Named { page, continuation } => {
                    let mut occurrences = Vec::new();
                    occurrences.try_reserve_exact(page.occurrences().len())?;
                    for occurrence in page.occurrences() {
                        budget.charge(occurrence.source_label().len())?;
                        for segment in occurrence.path() {
                            if let pale_application::workspace::AuthoringInstancePathSegment::Field(field) = segment {
                                budget.charge(field.len())?;
                            }
                        }
                        occurrences.push(AuthoringOccurrence::from_application(occurrence));
                    }
                    OrganizationSourceMapping::Named {
                        occurrences,
                        inspected: u16::try_from(page.inspected())
                            .map_err(|_| OrganizationFailure::InternalCount)?,
                        continuation: continuation.map(|application| OrganizationSourceCursor {
                            record: self.clone(),
                            application,
                            maximum_inspected,
                            maximum_returned,
                            maximum_copied_bytes,
                        }),
                    }
                }
            };
            Ok(OrganizationSourcePage {
                record: self.clone(),
                mapping,
                copied_string_bytes: budget.used,
            })
        };
        execute().map_err(|error| failure(&self.diagnostic_identity, error))
    }
}

/// One copied shallow row and its independently retainable exact record handle.
#[derive(Clone, Debug)]
pub struct OrganizationEntry {
    row: OrganizationRow,
    handle: OrganizationRecordHandle,
}

impl OrganizationEntry {
    /// Shallow observed fields, excluding nested collections and geometry.
    #[must_use]
    pub const fn row(&self) -> &OrganizationRow {
        &self.row
    }
    /// Clone this handle to retain source/detail access independently of the page.
    #[must_use]
    pub const fn handle(&self) -> &OrganizationRecordHandle {
        &self.handle
    }
}

/// Bounded owned page, including explicit selected/empty/terminal distinctions.
///
/// Cloning copies projected values and shares the complete compilation. Raw
/// values are not admitted recipient payloads. Bytes are not canonicalized or
/// given a new public contract identity by this in-process API.
#[derive(Clone)]
pub struct OrganizationPage {
    owner: Arc<PortableWorkspaceUniverse>,
    entries: Vec<OrganizationEntry>,
    organization_selected: bool,
    inspected: u16,
    copied_string_bytes: u32,
    continuation: Option<OrganizationCursor>,
    endpoint_lookup: Option<OrganizationEndpointLookup>,
}

impl std::fmt::Debug for OrganizationPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationPage").finish_non_exhaustive()
    }
}

impl OrganizationPage {
    /// Actual workspace from the producing compilation.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.owner.workspace_id()
    }
    /// Principal-universe digest, not a reconstructible source-custody token.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        self.owner.principal_universe_digest()
    }
    /// Actual semantic IR digest; not editor generation or recipient context.
    #[must_use]
    pub fn semantic_ir_digest(&self) -> &str {
        self.owner.semantic_ir_digest()
    }
    /// Actual admission-evidence identity used by this compilation.
    #[must_use]
    pub fn admission_evidence_digest(&self) -> &str {
        self.owner.admission_evidence_digest()
    }
    /// Whether organization input was selected, independently of responsibility rows.
    #[must_use]
    pub const fn organization_selected(&self) -> bool {
        self.organization_selected
    }
    /// Bounded shallow rows in the application's canonical family/record order.
    #[must_use]
    pub fn entries(&self) -> &[OrganizationEntry] {
        &self.entries
    }
    /// Actual application record visits, not nested expansion or byte work.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }
    /// Cumulative endpoint-record lookup only; not a record's effective or native state.
    #[must_use]
    pub const fn endpoint_lookup(&self) -> Option<OrganizationEndpointLookup> {
        self.endpoint_lookup
    }
    /// UTF-8 string payload copied into these rows, excluding custody metadata.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> u32 {
        self.copied_string_bytes
    }
    /// None only after the application established the selected query's exhaustion.
    #[must_use]
    pub const fn continuation(&self) -> Option<&OrganizationCursor> {
        self.continuation.as_ref()
    }
    /// Whether this query has proved exhaustion; not complete World coverage.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.continuation.is_none()
    }
    /// Exact producing compilation, including its source maps.
    #[must_use]
    pub fn belongs_to(&self, compilation: &PortableWorkspaceCompilation) -> bool {
        Arc::ptr_eq(&self.owner, &compilation.universe)
    }
}

/// Exact record/source continuation, retaining the complete outer owner.
#[derive(Clone)]
pub struct OrganizationSourceCursor {
    record: OrganizationRecordHandle,
    application: ApplicationSourceCursor,
    maximum_inspected: u16,
    maximum_returned: u16,
    maximum_copied_bytes: u32,
}

impl std::fmt::Debug for OrganizationSourceCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationSourceCursor")
            .finish_non_exhaustive()
    }
}

/// Existing record-level source evidence; unmapped is not absent source.
#[derive(Clone, Debug)]
pub enum OrganizationSourceMapping {
    /// The complete record has no supported exact named mapping.
    UnmappedRecord,
    /// Actual matches and progress from the authoring owner.
    Named {
        /// Existing SDK source/role projection, with original authored positions.
        occurrences: Vec<AuthoringOccurrence>,
        /// Index candidates visited, including nonmatches.
        inspected: u16,
        /// None only after source scan exhaustion was established.
        continuation: Option<OrganizationSourceCursor>,
    },
}

/// Owned source evidence retaining its exact record even if no cursor is needed.
#[derive(Clone, Debug)]
pub struct OrganizationSourcePage {
    record: OrganizationRecordHandle,
    mapping: OrganizationSourceMapping,
    copied_string_bytes: u32,
}

/// Opaque continuation that resumes only through its admitted inspection owner.
#[derive(Clone)]
pub struct RecipientQualifiedOrganizationCursor {
    pub(super) inner: OrganizationCursor,
}

/// Exact record custody without a supplied-input source or detail method.
#[derive(Clone)]
pub struct RecipientQualifiedOrganizationRecordHandle {
    pub(super) inner: OrganizationRecordHandle,
}

impl RecipientQualifiedOrganizationRecordHandle {
    /// Whether both handles select one record under one exact compiler owner.
    #[must_use]
    pub fn same_record(&self, other: &Self) -> bool {
        self.inner.same_record(&other.inner)
    }

    /// Workspace coordinate retained by this exact record.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.inner.workspace_id()
    }
}

/// One admitted shallow row and its authority-bound nested record custody.
#[derive(Clone)]
pub struct RecipientQualifiedOrganizationEntry {
    row: OrganizationRow,
    handle: RecipientQualifiedOrganizationRecordHandle,
}

impl RecipientQualifiedOrganizationEntry {
    /// Shallow semantic fields, excluding renderer state and nested collections.
    #[must_use]
    pub const fn row(&self) -> &OrganizationRow {
        &self.row
    }

    /// Exact record used for another freshly admitted source query.
    #[must_use]
    pub const fn handle(&self) -> &RecipientQualifiedOrganizationRecordHandle {
        &self.handle
    }
}

/// Admitted organization page whose continuation cannot bypass authority checks.
pub struct RecipientQualifiedOrganizationPage {
    entries: Vec<RecipientQualifiedOrganizationEntry>,
    workspace_id: String,
    principal_universe_digest: String,
    semantic_ir_digest: String,
    admission_evidence_digest: String,
    organization_selected: bool,
    inspected: u16,
    copied_string_bytes: u32,
    endpoint_lookup: Option<OrganizationEndpointLookup>,
    continuation: Option<RecipientQualifiedOrganizationCursor>,
}

impl RecipientQualifiedOrganizationPage {
    /// Workspace that owns the admitted semantic result.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }
    /// Principal-relative universe identity.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        &self.principal_universe_digest
    }
    /// Semantic identity of the producing compilation.
    #[must_use]
    pub fn semantic_ir_digest(&self) -> &str {
        &self.semantic_ir_digest
    }
    /// Admission-evidence identity of the producing compilation.
    #[must_use]
    pub fn admission_evidence_digest(&self) -> &str {
        &self.admission_evidence_digest
    }
    /// Whether organization input was selected.
    #[must_use]
    pub const fn organization_selected(&self) -> bool {
        self.organization_selected
    }
    /// Admitted shallow rows in canonical order.
    #[must_use]
    pub fn entries(&self) -> &[RecipientQualifiedOrganizationEntry] {
        &self.entries
    }
    /// Record cells visited by this query.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }
    /// Cumulative endpoint result for endpoint queries.
    #[must_use]
    pub const fn endpoint_lookup(&self) -> Option<OrganizationEndpointLookup> {
        self.endpoint_lookup
    }
    /// UTF-8 string bytes copied into this page.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> u32 {
        self.copied_string_bytes
    }
    /// Resume only through the same qualified inspection owner.
    #[must_use]
    pub const fn continuation(&self) -> Option<&RecipientQualifiedOrganizationCursor> {
        self.continuation.as_ref()
    }
    /// Whether enumeration has proved exhaustion.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.continuation.is_none()
    }
}

/// Opaque continuation for one freshly admitted record-source query.
#[derive(Clone, Debug)]
pub struct RecipientQualifiedOrganizationSourceCursor {
    inner: OrganizationSourceCursor,
}

/// Admitted source mapping without a supplied-input continuation.
#[derive(Clone, Debug)]
pub enum RecipientQualifiedOrganizationSourceMapping {
    /// No supported exact named mapping for this record.
    UnmappedRecord,
    /// Exact named occurrences and source-scan progress.
    Named {
        /// Compiler-owned source occurrences.
        occurrences: Vec<AuthoringOccurrence>,
        /// Source candidates examined, including nonmatches.
        inspected: u16,
        /// Resume only through the same qualified inspection owner.
        continuation: Option<RecipientQualifiedOrganizationSourceCursor>,
    },
}

/// Admitted source evidence retaining its exact qualified record.
pub struct RecipientQualifiedOrganizationSourcePage {
    record: RecipientQualifiedOrganizationRecordHandle,
    mapping: RecipientQualifiedOrganizationSourceMapping,
    copied_string_bytes: u32,
}

impl RecipientQualifiedOrganizationSourcePage {
    /// Exact record that supplied this mapping.
    #[must_use]
    pub const fn record(&self) -> &RecipientQualifiedOrganizationRecordHandle {
        &self.record
    }
    /// Unmapped or exact named source evidence.
    #[must_use]
    pub const fn mapping(&self) -> &RecipientQualifiedOrganizationSourceMapping {
        &self.mapping
    }
    /// UTF-8 source-location bytes copied into this page.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> u32 {
        self.copied_string_bytes
    }
}

impl OrganizationSourcePage {
    /// Exact independently retained record, not an inferred source identity.
    #[must_use]
    pub const fn record(&self) -> &OrganizationRecordHandle {
        &self.record
    }
    /// Distinct unmapped, named empty, partial and terminal source result.
    #[must_use]
    pub const fn mapping(&self) -> &OrganizationSourceMapping {
        &self.mapping
    }
    /// Returned source-label/path-field UTF-8 payload, not total allocation.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> u32 {
        self.copied_string_bytes
    }
}

impl PortableWorkspaceCompilation {
    /// Copy a bounded shallow organizational page from this admitted input.
    ///
    /// Every raw record limit is explicit 1..=256; zero has no default here.
    /// Copy bytes lie in `1..=MAX_ORGANIZATION_COPY_BYTES` and count only owned
    /// string payload. Cells, allocator overhead, cursor/diagnostic metadata,
    /// source-index work and retention of the complete compilation are separate.
    /// No nested collection is silently copied or truncated in a header.
    ///
    /// # Errors
    /// Query/custody errors precede allocation. Copy exhaustion publishes no
    /// page or advanced cursor. Retrying requires the original exact query;
    /// choose adequate limits for the desired page before starting enumeration.
    /// This is supplied-input inspection, not authenticated World delivery.
    pub fn inspect_organization(
        &self,
        after: Option<&OrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<OrganizationPage, ApiError> {
        let execute = || -> Result<OrganizationPage, OrganizationFailure> {
            self.validate_organization_cursor(
                after,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )?;
            let page = self.universe.inspect_organization(
                after.map(|cursor| &cursor.application),
                maximum_inspected,
                maximum_returned,
            )?;
            self.project_organization_page(
                page,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )
        };
        execute().map_err(|error| failure(&self.diagnostic_identity, error))
    }

    pub(super) fn validate_organization_cursor(
        &self,
        after: Option<&OrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<(), OrganizationFailure> {
        validate_limits(maximum_inspected, maximum_returned, maximum_copied_bytes)?;
        if after.is_some_and(|cursor| {
            !Arc::ptr_eq(&self.universe, &cursor.owner)
                || cursor.maximum_inspected != maximum_inspected
                || cursor.maximum_returned != maximum_returned
                || cursor.maximum_copied_bytes != maximum_copied_bytes
        }) {
            return Err(OrganizationFailure::ContinuationMismatch);
        }
        Ok(())
    }

    pub(super) fn project_organization_page(
        &self,
        page: pale_application::workspace::OrganizationInspectionPage<'_>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<OrganizationPage, OrganizationFailure> {
        let mut entries = Vec::new();
        entries.try_reserve_exact(page.entries().len())?;
        let diagnostic_identity = Arc::new(self.diagnostic_identity.clone());
        let mut budget = CopyBudget::new(maximum_copied_bytes);
        for entry in page.entries() {
            entries.push(project_entry(
                &self.universe,
                entry,
                &diagnostic_identity,
                &mut budget,
            )?);
        }
        Ok(OrganizationPage {
            owner: Arc::clone(&self.universe),
            entries,
            organization_selected: page.organization_selected(),
            inspected: page.inspected(),
            endpoint_lookup: page
                .endpoint_lookup()
                .map(OrganizationEndpointLookup::from_application),
            copied_string_bytes: budget.used,
            continuation: page
                .continuation()
                .cloned()
                .map(|application| OrganizationCursor {
                    owner: Arc::clone(&self.universe),
                    application,
                    maximum_inspected,
                    maximum_returned,
                    maximum_copied_bytes,
                }),
        })
    }
}

impl super::RecipientQualifiedWorkspaceInspection {
    /// Copy one bounded organization page after application-owned recipient admission.
    ///
    /// Admission runs before selector, continuation, or SDK copy-limit validation.
    /// A continuation therefore returns to the same authority owner before Pale
    /// reveals whether that opaque position is valid.
    ///
    /// # Errors
    /// Returns an admission/budget failure or the existing SDK projection failure.
    pub fn inspect_organization(
        &self,
        after: Option<&RecipientQualifiedOrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<RecipientQualifiedOrganizationPage, super::RecipientQualifiedWorkspaceInspectionError>
    {
        let page = self
            .admitted
            .inspect_organization(
                after.map(|cursor| &cursor.inner.application),
                maximum_inspected,
                maximum_returned,
            )?
            .map_err(|source| {
                failure(
                    &self.compilation.diagnostic_identity,
                    OrganizationFailure::Application(source),
                )
            })?;
        self.compilation
            .validate_organization_cursor(
                after.map(|cursor| &cursor.inner),
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )
            .map_err(|source| failure(&self.compilation.diagnostic_identity, source))?;
        let result = self
            .compilation
            .project_organization_page(
                page,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )
            .map_err(|source| failure(&self.compilation.diagnostic_identity, source))?;
        self.admitted
            .budget()
            .charge_output(u64::from(result.copied_string_bytes()), 0)
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(qualify_page(result))
    }

    /// Find an exact endpoint after fresh recipient admission.
    ///
    /// # Errors
    /// Returns an admission, selector, custody, budget, or projection failure.
    pub fn inspect_organization_endpoint(
        &self,
        endpoint: &OrganizationNode,
        after: Option<&RecipientQualifiedOrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<RecipientQualifiedOrganizationPage, super::RecipientQualifiedWorkspaceInspectionError>
    {
        self.admitted
            .preflight()
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        let endpoint = endpoint
            .application_selector()
            .map_err(|source| failure(&self.compilation.diagnostic_identity, source))?;
        let page = self
            .admitted
            .inspect_organization_endpoint(
                &endpoint,
                after.map(|cursor| &cursor.inner.application),
                maximum_inspected,
                maximum_returned,
            )?
            .map_err(|source| {
                failure(
                    &self.compilation.diagnostic_identity,
                    OrganizationFailure::Application(source),
                )
            })?;
        self.qualify_application_page(
            page,
            after.map(|cursor| &cursor.inner),
            maximum_inspected,
            maximum_returned,
            maximum_copied_bytes,
        )
    }

    /// Page relations incident to one exact endpoint after fresh admission.
    ///
    /// # Errors
    /// Returns an admission, selector, custody, budget, or projection failure.
    pub fn inspect_organization_incident_relations(
        &self,
        endpoint: &OrganizationNode,
        after: Option<&RecipientQualifiedOrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<RecipientQualifiedOrganizationPage, super::RecipientQualifiedWorkspaceInspectionError>
    {
        self.admitted
            .preflight()
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        let endpoint = endpoint
            .application_selector()
            .map_err(|source| failure(&self.compilation.diagnostic_identity, source))?;
        let page = self
            .admitted
            .inspect_organization_incident_relations(
                &endpoint,
                after.map(|cursor| &cursor.inner.application),
                maximum_inspected,
                maximum_returned,
            )?
            .map_err(|source| {
                failure(
                    &self.compilation.diagnostic_identity,
                    OrganizationFailure::Application(source),
                )
            })?;
        self.qualify_application_page(
            page,
            after.map(|cursor| &cursor.inner),
            maximum_inspected,
            maximum_returned,
            maximum_copied_bytes,
        )
    }

    /// Page services incident to one exact scope after fresh admission.
    ///
    /// # Errors
    /// Returns an admission, selector, custody, budget, or projection failure.
    pub fn inspect_organization_incident_services(
        &self,
        scope: &OrganizationScopeRef,
        after: Option<&RecipientQualifiedOrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<RecipientQualifiedOrganizationPage, super::RecipientQualifiedWorkspaceInspectionError>
    {
        self.admitted
            .preflight()
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        let scope = scope
            .application_selector()
            .map_err(|source| failure(&self.compilation.diagnostic_identity, source))?;
        let page = self
            .admitted
            .inspect_organization_incident_services(
                &scope,
                after.map(|cursor| &cursor.inner.application),
                maximum_inspected,
                maximum_returned,
            )?
            .map_err(|source| {
                failure(
                    &self.compilation.diagnostic_identity,
                    OrganizationFailure::Application(source),
                )
            })?;
        self.qualify_application_page(
            page,
            after.map(|cursor| &cursor.inner),
            maximum_inspected,
            maximum_returned,
            maximum_copied_bytes,
        )
    }

    /// Read exact source evidence after fresh recipient admission.
    ///
    /// # Errors
    /// Returns an admission, custody, budget, or projection failure.
    pub fn inspect_organization_sources(
        &self,
        record: &RecipientQualifiedOrganizationRecordHandle,
        after: Option<&RecipientQualifiedOrganizationSourceCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<
        RecipientQualifiedOrganizationSourcePage,
        super::RecipientQualifiedWorkspaceInspectionError,
    > {
        let permit = self.admitted.authorize_organization_sources(
            &record.inner.application,
            after.is_some(),
            maximum_inspected,
            maximum_returned,
        )?;
        debug_assert!(permit.matches(
            &record.inner.application,
            after.is_some(),
            maximum_inspected,
            maximum_returned,
        ));
        let page = record.inner.sources(
            after.map(|cursor| &cursor.inner),
            maximum_inspected,
            maximum_returned,
            maximum_copied_bytes,
        )?;
        self.admitted
            .budget()
            .charge_output(u64::from(page.copied_string_bytes()), 0)
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        let mapping = match page.mapping() {
            OrganizationSourceMapping::UnmappedRecord => {
                RecipientQualifiedOrganizationSourceMapping::UnmappedRecord
            }
            OrganizationSourceMapping::Named {
                occurrences,
                inspected,
                continuation,
            } => RecipientQualifiedOrganizationSourceMapping::Named {
                occurrences: occurrences.clone(),
                inspected: *inspected,
                continuation: continuation
                    .clone()
                    .map(|inner| RecipientQualifiedOrganizationSourceCursor { inner }),
            },
        };
        Ok(RecipientQualifiedOrganizationSourcePage {
            record: record.clone(),
            mapping,
            copied_string_bytes: page.copied_string_bytes(),
        })
    }

    fn qualify_application_page(
        &self,
        page: pale_application::workspace::OrganizationInspectionPage<'_>,
        after: Option<&OrganizationCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_bytes: u32,
    ) -> Result<RecipientQualifiedOrganizationPage, super::RecipientQualifiedWorkspaceInspectionError>
    {
        self.compilation
            .validate_organization_cursor(
                after,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )
            .map_err(|source| failure(&self.compilation.diagnostic_identity, source))?;
        let result = self
            .compilation
            .project_organization_page(
                page,
                maximum_inspected,
                maximum_returned,
                maximum_copied_bytes,
            )
            .map_err(|source| failure(&self.compilation.diagnostic_identity, source))?;
        self.admitted
            .budget()
            .charge_output(u64::from(result.copied_string_bytes()), 0)
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(qualify_page(result))
    }
}

fn qualify_page(page: OrganizationPage) -> RecipientQualifiedOrganizationPage {
    let entries = page
        .entries()
        .iter()
        .map(|entry| RecipientQualifiedOrganizationEntry {
            row: entry.row().clone(),
            handle: RecipientQualifiedOrganizationRecordHandle {
                inner: entry.handle().clone(),
            },
        })
        .collect();
    RecipientQualifiedOrganizationPage {
        entries,
        workspace_id: page.workspace_id().to_owned(),
        principal_universe_digest: page.principal_universe_digest().to_owned(),
        semantic_ir_digest: page.semantic_ir_digest().to_owned(),
        admission_evidence_digest: page.admission_evidence_digest().to_owned(),
        organization_selected: page.organization_selected(),
        inspected: page.inspected(),
        copied_string_bytes: page.copied_string_bytes(),
        endpoint_lookup: page.endpoint_lookup(),
        continuation: page
            .continuation()
            .cloned()
            .map(|inner| RecipientQualifiedOrganizationCursor { inner }),
    }
}

fn project_entry(
    owner: &Arc<PortableWorkspaceUniverse>,
    entry: &pale_application::workspace::OrganizationInspectionEntry<'_>,
    diagnostic_identity: &Arc<RecordSchemaIdentity>,
    budget: &mut CopyBudget,
) -> Result<OrganizationEntry, OrganizationFailure> {
    Ok(OrganizationEntry {
        row: projection::row(entry.record(), budget)?,
        handle: OrganizationRecordHandle {
            owner: Arc::clone(owner),
            application: entry.retain(),
            diagnostic_identity: Arc::clone(diagnostic_identity),
        },
    })
}

struct CopyBudget {
    maximum: u32,
    used: u32,
}

impl CopyBudget {
    const fn new(maximum: u32) -> Self {
        Self { maximum, used: 0 }
    }
    fn charge(&mut self, bytes: usize) -> Result<(), OrganizationFailure> {
        let bytes = u32::try_from(bytes).map_err(|_| OrganizationFailure::CopyBudget)?;
        self.used = self
            .used
            .checked_add(bytes)
            .filter(|used| *used <= self.maximum)
            .ok_or(OrganizationFailure::CopyBudget)?;
        Ok(())
    }
    fn text(&mut self, value: &str) -> Result<String, OrganizationFailure> {
        self.charge(value.len())?;
        let mut text = String::new();
        text.try_reserve_exact(value.len())?;
        text.push_str(value);
        Ok(text)
    }
}

fn validate_limits(
    inspected: u16,
    returned: u16,
    copied_bytes: u32,
) -> Result<(), OrganizationFailure> {
    if !(1..=256).contains(&inspected)
        || !(1..=256).contains(&returned)
        || !(1..=MAX_ORGANIZATION_COPY_BYTES).contains(&copied_bytes)
    {
        return Err(OrganizationFailure::InvalidLimit);
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub(super) enum OrganizationFailure {
    #[error("organization endpoint contains an invalid typed identifier")]
    InvalidEndpoint,
    #[error("organization query limits are outside their bounded range")]
    InvalidLimit,
    #[error("organization continuation does not match its exact owner and raw limits")]
    ContinuationMismatch,
    #[error("organization copied-string payload exceeds the requested budget")]
    CopyBudget,
    #[error("organization projected count violates the application's bounded result")]
    InternalCount,
    #[error("organization inspection failed: {0}")]
    Application(#[from] ApplicationError),
    #[error("organization projection allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
}

pub(super) fn failure(identity: &RecordSchemaIdentity, source: OrganizationFailure) -> ApiError {
    let (code, stage, disposition, summary, retry) = match &source {
        OrganizationFailure::InvalidEndpoint => (
            "workspace_organization_invalid_endpoint",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "organization endpoint contains an invalid typed identifier",
            DiagnosticRetryV1::AfterCorrection,
        ),
        OrganizationFailure::Application(ApplicationError::SectionUnavailable) => (
            "workspace_organization_section_unavailable",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "organization record does not own the requested detail section",
            DiagnosticRetryV1::AfterCorrection,
        ),
        OrganizationFailure::InvalidLimit
        | OrganizationFailure::Application(ApplicationError::InvalidLimit) => (
            "workspace_organization_invalid_limit",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "organization query limits are outside their bounded range",
            DiagnosticRetryV1::AfterCorrection,
        ),
        OrganizationFailure::ContinuationMismatch
        | OrganizationFailure::Application(
            ApplicationError::ContinuationMismatch | ApplicationError::RecordOwnerMismatch,
        ) => (
            "workspace_organization_custody_mismatch",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "organization continuation or record does not match its exact owner and query",
            DiagnosticRetryV1::AfterCorrection,
        ),
        OrganizationFailure::CopyBudget => (
            "workspace_organization_copy_budget",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "organization copied-string payload exceeds the requested budget",
            DiagnosticRetryV1::AfterCorrection,
        ),
        OrganizationFailure::InternalCount
        | OrganizationFailure::Application(ApplicationError::InvalidPosition) => (
            "workspace_organization_position_invalid",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "organization inspection position or count violates its retained owner invariant",
            DiagnosticRetryV1::NotIndicated,
        ),
        OrganizationFailure::Allocation(_)
        | OrganizationFailure::Application(ApplicationError::Allocation(_)) => (
            "workspace_organization_allocation_failed",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "organization inspection could not reserve its bounded result",
            DiagnosticRetryV1::NotIndicated,
        ),
    };
    ApiError::new_with_opaque_source(
        identity,
        source,
        FailureSpecification {
            code,
            stage,
            disposition,
            summary,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            subject: None,
            schema_coordinate: None,
            location: None,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}
