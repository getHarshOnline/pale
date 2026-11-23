// Recipient-qualified workspace inspection. These classes have no JavaScript
// constructor. A trusted application or resident route must issue them.

fn recipient_inspection_error(
    env: &Env,
    error: pale_sdk::RecipientQualifiedWorkspaceInspectionError,
) -> napi::Error {
    match error {
        pale_sdk::RecipientQualifiedWorkspaceInspectionError::Projection(error) => {
            diagnostic_error(env, error)
        }
        pale_sdk::RecipientQualifiedWorkspaceInspectionError::Access(error) => {
            napi::Error::new(napi::Status::GenericFailure, error.to_string())
        }
    }
}

/// Opaque recipient-qualified access to one retained workspace.
///
/// Only an application or resident route can return this handle. JavaScript
/// cannot construct or copy its authority.
#[napi(js_name = "RecipientQualifiedWorkspaceInspection")]
pub struct RecipientQualifiedWorkspaceInspection {
    pub(crate) inner: pale_sdk::RecipientQualifiedWorkspaceInspection,
}

/// Identity-and-count projection of a recipient-qualified workspace.
#[napi(js_name = "RecipientQualifiedWorkspaceSummary")]
pub struct RecipientQualifiedWorkspaceSummary {
    inner: pale_sdk::PortableWorkspaceCompilationSummary,
}

/// Identity summaries for one recipient-qualified ordered comparison.
#[napi(js_name = "RecipientQualifiedWorkspacePairSummary")]
pub struct RecipientQualifiedWorkspacePairSummary {
    inner: pale_sdk::RecipientQualifiedWorkspacePairSummary,
}

#[napi]
impl RecipientQualifiedWorkspaceSummary {
    /// Complete API-owned canonical summary bytes.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        Buffer::from(self.inner.canonical_bytes().to_vec())
    }
    /// Workspace identity.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }
    /// Principal-relative universe identity.
    #[napi(getter)]
    pub fn principal_universe_digest(&self) -> String {
        self.inner.principal_universe_digest().to_owned()
    }
    /// Semantic identity.
    #[napi(getter)]
    pub fn semantic_ir_digest(&self) -> String {
        self.inner.semantic_ir_digest().to_owned()
    }
    /// Admission-evidence identity.
    #[napi(getter)]
    pub fn admission_evidence_digest(&self) -> String {
        self.inner.admission_evidence_digest().to_owned()
    }
    /// Source documents consumed at ingress.
    #[napi(getter)]
    pub fn source_document_count(&self) -> u32 {
        self.inner.source_document_count()
    }
    /// Retained causal source coordinates.
    #[napi(getter)]
    pub fn derivation_source_count(&self) -> u32 {
        self.inner.derivation_source_count()
    }
    /// Retained causal derivation nodes.
    #[napi(getter)]
    pub fn derivation_node_count(&self) -> u32 {
        self.inner.derivation_node_count()
    }
    /// Owner-proof records consumed at admission.
    #[napi(getter)]
    pub fn admission_evidence_count(&self) -> u32 {
        self.inner.admission_evidence_count()
    }
    /// Adopted module releases in the admitted closure.
    #[napi(getter)]
    pub fn adopted_release_count(&self) -> u32 {
        self.inner.adopted_release_count()
    }
    /// Package releases in the admitted closure.
    #[napi(getter)]
    pub fn package_release_count(&self) -> u32 {
        self.inner.package_release_count()
    }
    /// Attestor-scoped normative-Pack releases in the admitted closure.
    #[napi(getter)]
    pub fn normative_pack_release_count(&self) -> u32 {
        self.inner.normative_pack_release_count()
    }
}

/// Resume organization enumeration only through its qualified owner.
#[napi(js_name = "RecipientQualifiedOrganizationCursor")]
pub struct RecipientQualifiedOrganizationCursor {
    inner: pale_sdk::RecipientQualifiedOrganizationCursor,
}

/// Exact admitted organization record with no supplied-input query method.
#[napi(js_name = "RecipientQualifiedOrganizationRecordHandle")]
pub struct RecipientQualifiedOrganizationRecordHandle {
    inner: pale_sdk::RecipientQualifiedOrganizationRecordHandle,
}

/// One admitted organization row and its qualified record custody.
#[napi(js_name = "RecipientQualifiedOrganizationEntry")]
pub struct RecipientQualifiedOrganizationEntry {
    inner: pale_sdk::RecipientQualifiedOrganizationEntry,
}

/// One admitted organization page with an authority-bound continuation.
#[napi(js_name = "RecipientQualifiedOrganizationPage")]
pub struct RecipientQualifiedOrganizationPage {
    inner: pale_sdk::RecipientQualifiedOrganizationPage,
}

/// Resume a source query only through its qualified owner.
#[napi(js_name = "RecipientQualifiedOrganizationSourceCursor")]
pub struct RecipientQualifiedOrganizationSourceCursor {
    inner: pale_sdk::RecipientQualifiedOrganizationSourceCursor,
}

/// Exact admitted source evidence for one qualified record.
#[napi(js_name = "RecipientQualifiedOrganizationSourcePage")]
pub struct RecipientQualifiedOrganizationSourcePage {
    inner: pale_sdk::RecipientQualifiedOrganizationSourcePage,
}

/// Exact source mapping without a supplied-input continuation.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum RecipientQualifiedOrganizationSourceMapping {
    /// No supported exact named mapping for this record.
    UnmappedRecord,
    /// Named source occurrences and an authority-bound continuation.
    Named {
        /// Compiler-owned source occurrences.
        occurrences: Vec<AuthoringOccurrence>,
        /// Source candidates examined, including nonmatches.
        inspected: u16,
        /// Null only after this source scan is complete.
        continuation: Option<RecipientQualifiedOrganizationSourceCursor>,
    },
}

#[napi]
impl RecipientQualifiedWorkspaceInspection {
    /// Identity-and-count projection of the retained workspace.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure before projection.
    #[napi]
    pub fn summary(&self) -> Result<RecipientQualifiedWorkspaceSummary> {
        self.inner
            .summary()
            .map(|inner| RecipientQualifiedWorkspaceSummary { inner })
            .map_err(|error| napi::Error::new(napi::Status::GenericFailure, error.to_string()))
    }

    /// Page admitted organization records. Every continuation is re-authorized.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or SDK projection failure.
    #[napi]
    pub fn inspect_organization(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&RecipientQualifiedOrganizationCursor>,
    ) -> Result<RecipientQualifiedOrganizationPage> {
        self.inner
            .inspect_organization(
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| RecipientQualifiedOrganizationPage { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }

    /// Find one endpoint after recipient admission.
    ///
    /// # Errors
    /// Returns an authority, selector, budget, custody, or projection failure.
    #[napi]
    pub fn inspect_organization_endpoint(
        &self,
        env: Env,
        endpoint: OrganizationNode,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&RecipientQualifiedOrganizationCursor>,
    ) -> Result<RecipientQualifiedOrganizationPage> {
        self.inner
            .inspect_organization_endpoint(
                &endpoint.into_api(),
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| RecipientQualifiedOrganizationPage { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }

    /// Page admitted relations incident to one endpoint.
    ///
    /// # Errors
    /// Returns an authority, selector, budget, custody, or projection failure.
    #[napi]
    pub fn inspect_organization_incident_relations(
        &self,
        env: Env,
        endpoint: OrganizationNode,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&RecipientQualifiedOrganizationCursor>,
    ) -> Result<RecipientQualifiedOrganizationPage> {
        self.inner
            .inspect_organization_incident_relations(
                &endpoint.into_api(),
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| RecipientQualifiedOrganizationPage { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }

    /// Page admitted services incident to one scope.
    ///
    /// # Errors
    /// Returns an authority, selector, budget, custody, or projection failure.
    #[napi]
    pub fn inspect_organization_incident_services(
        &self,
        env: Env,
        scope: OrganizationScopeRef,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&RecipientQualifiedOrganizationCursor>,
    ) -> Result<RecipientQualifiedOrganizationPage> {
        self.inner
            .inspect_organization_incident_services(
                &scope.into_api(),
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| RecipientQualifiedOrganizationPage { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }

    /// Associate an observation only after recipient authority is current.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure. Coordinate and source
    /// refusals remain values on the returned association result.
    #[napi]
    pub fn associate_profile_observation(
        &self,
        snapshot: &crate::ObservationSnapshot,
        profile: &crate::EffectiveProfileCompilation,
    ) -> Result<crate::ObservationProfileAssociationResult> {
        snapshot
            .inner
            .associate_recipient_qualified_profile(&profile.inner, &self.inner)
            .map(|inner| crate::ObservationProfileAssociationResult { inner })
            .map_err(|error| napi::Error::new(napi::Status::GenericFailure, error.to_string()))
    }
}

#[napi]
impl RecipientQualifiedOrganizationPage {
    /// Cumulative endpoint state, or null for enumeration and incident queries.
    #[napi(getter)]
    pub fn endpoint_lookup(&self) -> Option<OrganizationEndpointLookup> {
        self.inner
            .endpoint_lookup()
            .map(OrganizationEndpointLookup::from_api)
    }
    /// Copy the admitted rows and retain their qualified record custody.
    #[napi]
    pub fn entries(&self) -> Vec<RecipientQualifiedOrganizationEntry> {
        self.inner
            .entries()
            .iter()
            .cloned()
            .map(|inner| RecipientQualifiedOrganizationEntry { inner })
            .collect()
    }
    /// Workspace that owns this semantic result.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }
    /// Principal-relative universe identity.
    #[napi(getter)]
    pub fn principal_universe_digest(&self) -> String {
        self.inner.principal_universe_digest().to_owned()
    }
    /// Semantic identity of the producing compilation.
    #[napi(getter)]
    pub fn semantic_ir_digest(&self) -> String {
        self.inner.semantic_ir_digest().to_owned()
    }
    /// Admission-evidence identity of the producing compilation.
    #[napi(getter)]
    pub fn admission_evidence_digest(&self) -> String {
        self.inner.admission_evidence_digest().to_owned()
    }
    /// Whether organization input was selected.
    #[napi(getter)]
    pub fn organization_selected(&self) -> bool {
        self.inner.organization_selected()
    }
    /// Record cells visited by this query.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }
    /// UTF-8 string bytes copied into this page.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }
    /// Whether the selected enumeration is complete.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }
    /// Resume only through the same qualified inspection owner.
    #[napi]
    pub fn continuation(&self) -> Option<RecipientQualifiedOrganizationCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| RecipientQualifiedOrganizationCursor { inner })
    }
}

#[napi]
impl RecipientQualifiedOrganizationEntry {
    /// Copy the admitted semantic row.
    #[napi(getter)]
    pub fn row(&self) -> OrganizationRow {
        OrganizationRow::from_api(self.inner.row())
    }
    /// Retain the exact record for another admitted source query.
    #[napi(getter)]
    pub fn handle(&self) -> RecipientQualifiedOrganizationRecordHandle {
        RecipientQualifiedOrganizationRecordHandle {
            inner: self.inner.handle().clone(),
        }
    }
}

#[napi]
impl RecipientQualifiedOrganizationRecordHandle {
    /// Test exact record custody under one compiler owner.
    #[napi]
    pub fn same_record(&self, other: &RecipientQualifiedOrganizationRecordHandle) -> bool {
        self.inner.same_record(&other.inner)
    }
    /// Workspace retained by this record.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }
}

#[napi]
impl RecipientQualifiedWorkspaceInspection {
    /// Read source evidence for an admitted record. Every continuation is re-authorized.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or projection failure.
    #[napi]
    pub fn inspect_organization_sources(
        &self,
        env: Env,
        record: &RecipientQualifiedOrganizationRecordHandle,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&RecipientQualifiedOrganizationSourceCursor>,
    ) -> Result<RecipientQualifiedOrganizationSourcePage> {
        self.inner
            .inspect_organization_sources(
                &record.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| RecipientQualifiedOrganizationSourcePage { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }
}

#[napi]
impl RecipientQualifiedOrganizationSourcePage {
    /// Exact admitted record that supplied this page.
    #[napi(getter)]
    pub fn record(&self) -> RecipientQualifiedOrganizationRecordHandle {
        RecipientQualifiedOrganizationRecordHandle {
            inner: self.inner.record().clone(),
        }
    }
    /// UTF-8 source-location bytes copied into this page.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }
    /// Copy the exact source mapping and qualified continuation.
    ///
    /// # Errors
    /// Returns a projection failure if a source index cannot fit the Node ABI.
    #[napi(getter)]
    pub fn mapping(&self) -> Result<RecipientQualifiedOrganizationSourceMapping> {
        Ok(match self.inner.mapping() {
            pale_sdk::RecipientQualifiedOrganizationSourceMapping::UnmappedRecord => {
                RecipientQualifiedOrganizationSourceMapping::UnmappedRecord
            }
            pale_sdk::RecipientQualifiedOrganizationSourceMapping::Named {
                occurrences,
                inspected,
                continuation,
            } => RecipientQualifiedOrganizationSourceMapping::Named {
                occurrences: occurrences
                    .iter()
                    .map(AuthoringOccurrence::from_api)
                    .collect::<Result<_>>()?,
                inspected: *inspected,
                continuation: continuation
                    .clone()
                    .map(|inner| RecipientQualifiedOrganizationSourceCursor { inner }),
            },
        })
    }
}

/// Opaque recipient-qualified access to one exact workspace comparison.
///
/// Only an application or resident route can return this handle. JavaScript
/// cannot construct or copy its authority.
#[napi(js_name = "RecipientQualifiedWorkspaceComparisonInspection")]
pub struct RecipientQualifiedWorkspaceComparisonInspection {
    pub(crate) inner: pale_sdk::RecipientQualifiedWorkspaceComparisonInspection,
}

/// Resume admitted subject enumeration only through its qualified owner.
#[napi(js_name = "RecipientQualifiedComparisonSubjectCursor")]
pub struct RecipientQualifiedComparisonSubjectCursor {
    inner: pale_sdk::RecipientQualifiedComparisonSubjectCursor,
}

/// Admitted comparison subjects with an authority-bound continuation.
#[napi(js_name = "RecipientQualifiedComparisonSubjectPage")]
pub struct RecipientQualifiedComparisonSubjectPage {
    inner: pale_sdk::RecipientQualifiedComparisonSubjectPage,
}

/// Resume admitted contributing roots only through the same qualified owner.
#[napi(js_name = "RecipientQualifiedContributingRootsCursor")]
pub struct RecipientQualifiedContributingRootsCursor {
    inner: pale_sdk::RecipientQualifiedContributingRootsCursor,
}

/// Admitted contributing roots with an authority-bound continuation.
#[napi(js_name = "RecipientQualifiedContributingRootsPage")]
pub struct RecipientQualifiedContributingRootsPage {
    inner: pale_sdk::RecipientQualifiedContributingRootsPage,
}

/// Resume admitted direct dependencies only through the same qualified owner.
#[napi(js_name = "RecipientQualifiedDependencyCursor")]
pub struct RecipientQualifiedDependencyCursor {
    inner: pale_sdk::RecipientQualifiedDependencyCursor,
}

/// Admitted direct dependencies with an authority-bound continuation.
#[napi(js_name = "RecipientQualifiedDependencyPage")]
pub struct RecipientQualifiedDependencyPage {
    inner: pale_sdk::RecipientQualifiedDependencyPage,
}

/// Resume admitted source-and-proof inspection only through the same owner.
#[napi(js_name = "RecipientQualifiedSubjectInspectionCursor")]
pub struct RecipientQualifiedSubjectInspectionCursor {
    inner: pale_sdk::RecipientQualifiedSubjectInspectionCursor,
}

/// Admitted proof and source evidence with an authority-bound continuation.
#[napi(js_name = "RecipientQualifiedSubjectInspection")]
pub struct RecipientQualifiedSubjectInspection {
    inner: pale_sdk::RecipientQualifiedSubjectInspection,
}

#[napi]
impl RecipientQualifiedWorkspaceComparisonInspection {
    /// Test whether this comparison retains these exact qualified inspections.
    ///
    /// This proves SDK custody only. It does not recheck recipient authority,
    /// disclosure, freshness, or the shared inspection budget.
    #[napi]
    pub fn compares_exact_inspections(
        &self,
        prior: &RecipientQualifiedWorkspaceInspection,
        current: &RecipientQualifiedWorkspaceInspection,
    ) -> bool {
        self.inner
            .compares_exact_inspections(&prior.inner, &current.inner)
    }

    /// Identity summaries of this exact admitted pair and comparison.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure before projection.
    #[napi]
    pub fn summaries(&self, env: Env) -> Result<RecipientQualifiedWorkspacePairSummary> {
        self.inner
            .summaries()
            .map(|inner| RecipientQualifiedWorkspacePairSummary { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }
    /// Enumerate admitted subject identities across both sides.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or projection failure.
    #[napi]
    pub fn subjects(
        &self,
        env: Env,
        limit: f64,
        after: Option<&RecipientQualifiedComparisonSubjectCursor>,
    ) -> Result<RecipientQualifiedComparisonSubjectPage> {
        self.inner
            .subjects(
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(limit)?,
            )
            .map(|inner| RecipientQualifiedComparisonSubjectPage { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }
    /// Project one admitted change-root page.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure.
    #[napi]
    pub fn change_roots(
        &self,
        env: Env,
        offset: u32,
        limit: u16,
    ) -> Result<WorkspaceChangeRootPage> {
        self.inner
            .change_roots(offset, limit)
            .map(WorkspaceChangeRootPage::from_api)
            .map_err(|error| recipient_inspection_error(&env, error))
    }
    /// Project one admitted impact page.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure.
    #[napi]
    pub fn impacts(&self, env: Env, offset: u32, limit: u16) -> Result<WorkspaceImpactPage> {
        self.inner
            .impacts(offset, limit)
            .map(WorkspaceImpactPage::from_api)
            .map_err(|error| recipient_inspection_error(&env, error))
    }
    /// Compare one exact subject after fresh admission.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure.
    #[napi]
    pub fn compare_subject(
        &self,
        env: Env,
        subject: &WorkspaceSubject,
    ) -> Result<WorkspaceSubjectComparison> {
        self.inner
            .compare_subject(&subject.inner)
            .map(|inner| WorkspaceSubjectComparison { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }
    /// Page admitted roots that contribute to one subject.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or projection failure.
    #[napi]
    pub fn contributing_roots(
        &self,
        env: Env,
        subject: &WorkspaceSubject,
        limit: f64,
        after: Option<&RecipientQualifiedContributingRootsCursor>,
    ) -> Result<RecipientQualifiedContributingRootsPage> {
        self.inner
            .contributing_roots(
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(limit)?,
            )
            .map(|inner| RecipientQualifiedContributingRootsPage { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }
    /// Read direct proof inputs from one admitted comparison side.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or projection failure.
    #[napi]
    pub fn direct_dependencies(
        &self,
        env: Env,
        side: OrganizationComparisonSide,
        subject: &WorkspaceSubject,
        limit: f64,
        after: Option<&RecipientQualifiedDependencyCursor>,
    ) -> Result<RecipientQualifiedDependencyPage> {
        self.inner
            .direct_dependencies(
                side.into_api(),
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(limit)?,
            )
            .map(|inner| RecipientQualifiedDependencyPage { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }
    /// Inspect source and proof evidence on one admitted comparison side.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or projection failure.
    #[napi]
    pub fn inspect_subject(
        &self,
        env: Env,
        side: OrganizationComparisonSide,
        subject: &WorkspaceSubject,
        input_limit: f64,
        maximum_inspected: f64,
        maximum_returned: f64,
        after: Option<&RecipientQualifiedSubjectInspectionCursor>,
    ) -> Result<RecipientQualifiedSubjectInspection> {
        self.inner
            .inspect_subject(
                side.into_api(),
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(input_limit)?,
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
            )
            .map(|inner| RecipientQualifiedSubjectInspection { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }
}

#[napi]
impl RecipientQualifiedWorkspacePairSummary {
    /// Summary of the exact predecessor owner.
    #[napi(getter)]
    pub fn prior(&self) -> RecipientQualifiedWorkspaceSummary {
        RecipientQualifiedWorkspaceSummary {
            inner: self.inner.prior().clone(),
        }
    }

    /// Summary of the exact current owner.
    #[napi(getter)]
    pub fn current(&self) -> RecipientQualifiedWorkspaceSummary {
        RecipientQualifiedWorkspaceSummary {
            inner: self.inner.current().clone(),
        }
    }

    /// Summary of the exact comparison derivation.
    #[napi(getter)]
    pub fn comparison(&self) -> WorkspaceComparisonSummary {
        WorkspaceComparisonSummary::from_api(self.inner.comparison().clone())
    }
}

#[napi]
impl RecipientQualifiedComparisonSubjectPage {
    /// Copy comparison coordinates and receipt identity.
    #[napi]
    pub fn comparison(&self) -> WorkspaceComparisonSummary {
        WorkspaceComparisonSummary::from_api(self.inner.comparison().clone())
    }
    /// Copy subject identities in canonical order.
    #[napi]
    pub fn subjects(&self) -> Vec<WorkspaceSubject> {
        self.inner
            .subjects()
            .iter()
            .map(WorkspaceSubject::from_api)
            .collect()
    }
    /// Native entries examined by the bounded merge.
    #[napi(getter)]
    pub fn inspected(&self) -> Result<u32> {
        u32::try_from(self.inner.inspected()).map_err(|_| {
            napi::Error::new(
                napi::Status::GenericFailure,
                "subject visit count exceeds the Node ABI",
            )
        })
    }
    /// UTF-8 bytes copied from subject identities.
    #[napi(getter)]
    pub fn subject_identity_bytes(&self) -> Result<u32> {
        u32::try_from(self.inner.subject_identity_bytes()).map_err(|_| {
            napi::Error::new(
                napi::Status::GenericFailure,
                "subject identity bytes exceed the Node ABI",
            )
        })
    }
    /// Resume only through the same qualified comparison owner.
    #[napi]
    pub fn continuation(&self) -> Option<RecipientQualifiedComparisonSubjectCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| RecipientQualifiedComparisonSubjectCursor { inner })
    }
    /// Copy the canonical projection without opaque custody.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        Buffer::from(self.inner.canonical_bytes().to_vec())
    }
}

#[napi]
impl RecipientQualifiedContributingRootsPage {
    /// Copy exact contributing roots.
    #[napi]
    pub fn roots(&self) -> Vec<WorkspaceChangeRoot> {
        self.inner
            .roots()
            .iter()
            .cloned()
            .map(|inner| WorkspaceChangeRoot { inner })
            .collect()
    }
    /// Resume only through the same qualified comparison owner.
    #[napi]
    pub fn continuation(&self) -> Option<RecipientQualifiedContributingRootsCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| RecipientQualifiedContributingRootsCursor { inner })
    }
    /// Copy the canonical projection without opaque custody.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        Buffer::from(self.inner.canonical_bytes().to_vec())
    }
}

#[napi]
impl RecipientQualifiedDependencyPage {
    /// Copy direct proof inputs.
    #[napi]
    pub fn dependencies(&self) -> Vec<WorkspaceCausalStep> {
        self.inner
            .dependencies()
            .iter()
            .cloned()
            .map(|inner| WorkspaceCausalStep { inner })
            .collect()
    }
    /// UTF-8 bytes copied into proof steps.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> Result<u32> {
        u32::try_from(self.inner.copied_string_bytes()).map_err(|_| {
            napi::Error::new(
                napi::Status::GenericFailure,
                "dependency bytes exceed the Node ABI",
            )
        })
    }
    /// Resume only through the same qualified comparison owner.
    #[napi]
    pub fn continuation(&self) -> Option<RecipientQualifiedDependencyCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| RecipientQualifiedDependencyCursor { inner })
    }
    /// Copy the canonical projection without opaque custody.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        Buffer::from(self.inner.canonical_bytes().to_vec())
    }
}

#[napi]
impl RecipientQualifiedSubjectInspection {
    /// Copy this page's direct proof inputs.
    #[napi]
    pub fn dependencies(&self) -> Vec<WorkspaceCausalStep> {
        self.inner
            .dependencies()
            .iter()
            .cloned()
            .map(|inner| WorkspaceCausalStep { inner })
            .collect()
    }
    /// Copy exact named source occurrences.
    ///
    /// # Errors
    /// Returns a projection error if a source index cannot fit the Node ABI.
    #[napi]
    pub fn occurrences(&self) -> Result<Vec<AuthoringOccurrence>> {
        self.inner
            .occurrences()
            .iter()
            .map(AuthoringOccurrence::from_api)
            .collect()
    }
    /// Source candidates examined by this call.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }
    /// UTF-8 bytes copied into proof steps.
    #[napi(getter)]
    pub fn dependency_copied_string_bytes(&self) -> u32 {
        self.inner.dependency_copied_string_bytes()
    }
    /// UTF-8 bytes copied into source locations.
    #[napi(getter)]
    pub fn source_copied_string_bytes(&self) -> u32 {
        self.inner.source_copied_string_bytes()
    }
    /// Resume only through the same qualified comparison owner.
    #[napi]
    pub fn continuation(&self) -> Option<RecipientQualifiedSubjectInspectionCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| RecipientQualifiedSubjectInspectionCursor { inner })
    }
    /// Copy the canonical projection without opaque custody.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        Buffer::from(self.inner.canonical_bytes().to_vec())
    }
}

/// Resume admitted organization correspondence only through its qualified owner.
#[napi(js_name = "RecipientQualifiedOrganizationCorrespondenceCursor")]
pub struct RecipientQualifiedOrganizationCorrespondenceCursor {
    inner: pale_sdk::RecipientQualifiedOrganizationCorrespondenceCursor,
}

/// Incomplete search, complete absence, or an admitted same-key comparison.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum RecipientQualifiedOrganizationCorrespondenceState {
    /// Resume through the same qualified comparison owner.
    Searching {
        /// Authority-bound continuation.
        cursor: RecipientQualifiedOrganizationCorrespondenceCursor,
    },
    /// The complete other-side family has no matching key.
    Unmatched,
    /// A same-key row was found and its complete value was compared.
    Matched {
        /// Admitted row from the other side.
        other: RecipientQualifiedOrganizationEntry,
        /// Complete typed value comparison.
        values: WorkspaceValueComparison,
    },
}

/// One admitted cross-generation organization lookup.
#[napi(js_name = "RecipientQualifiedOrganizationCorrespondence")]
pub struct RecipientQualifiedOrganizationCorrespondence {
    inner: pale_sdk::RecipientQualifiedOrganizationCorrespondence,
}

#[napi]
impl RecipientQualifiedWorkspaceComparisonInspection {
    /// Find one admitted organization record on the other comparison side.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or projection failure.
    #[napi]
    pub fn correspond_organization_record(
        &self,
        env: Env,
        selected_side: OrganizationComparisonSide,
        selected: &RecipientQualifiedOrganizationRecordHandle,
        maximum_inspected: f64,
        maximum_copied_bytes: f64,
        after: Option<&RecipientQualifiedOrganizationCorrespondenceCursor>,
    ) -> Result<RecipientQualifiedOrganizationCorrespondence> {
        self.inner
            .correspond_organization_record(
                selected_side.into_api(),
                &selected.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(maximum_inspected)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| RecipientQualifiedOrganizationCorrespondence { inner })
            .map_err(|error| recipient_inspection_error(&env, error))
    }
}

#[napi]
impl RecipientQualifiedOrganizationCorrespondence {
    /// Side that supplied the selected record.
    #[napi(getter)]
    pub fn selected_side(&self) -> OrganizationComparisonSide {
        OrganizationComparisonSide::from_api(self.inner.selected_side())
    }
    /// Retain the admitted selected record.
    #[napi(getter)]
    pub fn selected(&self) -> RecipientQualifiedOrganizationRecordHandle {
        RecipientQualifiedOrganizationRecordHandle {
            inner: self.inner.selected().clone(),
        }
    }
    /// Copy the bounded search state.
    #[napi(getter)]
    pub fn state(&self) -> RecipientQualifiedOrganizationCorrespondenceState {
        match self.inner.state() {
            pale_sdk::RecipientQualifiedOrganizationCorrespondenceState::Searching(cursor) => {
                RecipientQualifiedOrganizationCorrespondenceState::Searching {
                    cursor: RecipientQualifiedOrganizationCorrespondenceCursor {
                        inner: cursor.clone(),
                    },
                }
            }
            pale_sdk::RecipientQualifiedOrganizationCorrespondenceState::Unmatched => {
                RecipientQualifiedOrganizationCorrespondenceState::Unmatched
            }
            pale_sdk::RecipientQualifiedOrganizationCorrespondenceState::Matched {
                other,
                values,
            } => RecipientQualifiedOrganizationCorrespondenceState::Matched {
                other: RecipientQualifiedOrganizationEntry {
                    inner: other.as_ref().clone(),
                },
                values: match values {
                    pale_sdk::PortableWorkspaceValueComparison::Equal => {
                        WorkspaceValueComparison::Equal
                    }
                    pale_sdk::PortableWorkspaceValueComparison::Changed => {
                        WorkspaceValueComparison::Changed
                    }
                },
            },
        }
    }
    /// Other-side candidates examined.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }
    /// UTF-8 row bytes copied for a match.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }
}
