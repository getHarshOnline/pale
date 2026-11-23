// Retained organization handles over the existing supplied-input SDK owner.
// Queries enforce SDK limits once. Reading output getters makes another copy
// of bounded SDK values; the copied-string counter is the SDK query payload,
// not all Node/native allocations, repeated getter reads or retained memory.
// Handles retain semantic records and source mappings, not original source text.
// They grant no recipient disclosure, resident authority, freshness or effect.
use napi::bindgen_prelude::BigInt;

/// Reject numeric coercion; semantic budget validation remains with the SDK.
fn organization_copy_limit(value: f64) -> Result<u32> {
    if value.is_finite() && value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value) {
        Ok(value as u32)
    } else {
        Err(napi::Error::new(
            napi::Status::InvalidArg,
            "organization copy limit must be an integer from 0 through 4294967295",
        ))
    }
}

/// Resume organization enumeration with the original query limits.
///
/// Organization cursors and handles are issued by queries, with no public constructor
/// or serialized position. They retain the producing compilation's semantic records
/// and source mappings. Equal-valued compilations or members cannot substitute for
/// that owner; nested continuations also bind their parent, section and raw limits.
/// Retention does not grant recipient disclosure, resident authority or freshness.
#[napi]
pub struct OrganizationCursor {
    inner: pale_sdk::OrganizationCursor,
}

/// Retain one record for detail and source queries; see [`OrganizationCursor`] for custody.
#[napi]
pub struct OrganizationRecordHandle {
    inner: pale_sdk::OrganizationRecordHandle,
}

/// A shallow row paired with its independently retainable record handle.
#[napi]
pub struct OrganizationEntry {
    inner: pale_sdk::OrganizationEntry,
}

/// Bounded record results with compilation coordinates and an optional continuation.
#[napi]
pub struct OrganizationPage {
    inner: pale_sdk::OrganizationPage,
}

/// Record-membership evidence, separate from effective state and native readiness.
#[napi]
pub enum OrganizationEndpointLookup {
    /// No match yet; the scan can continue.
    SearchIncomplete,
    /// This or an earlier page found a record or binding witness.
    Found,
    /// The relevant scan ended without a local record.
    NotFound,
    /// An owner-native reference, not an invented local record.
    ExternalReference,
}

impl OrganizationEndpointLookup {
    fn from_api(value: pale_sdk::OrganizationEndpointLookup) -> Self {
        use pale_sdk::OrganizationEndpointLookup as V;
        match value {
            V::SearchIncomplete => Self::SearchIncomplete,
            V::Found => Self::Found,
            V::NotFound => Self::NotFound,
            V::ExternalReference => Self::ExternalReference,
        }
    }
}

/// Resume a record's source query; see [`OrganizationCursor`] for custody.
#[napi]
pub struct OrganizationSourceCursor {
    inner: pale_sdk::OrganizationSourceCursor,
}

/// Source matches or an unmapped record, with the queried record retained.
#[napi]
pub struct OrganizationSourcePage {
    inner: pale_sdk::OrganizationSourcePage,
}

/// Resume a detail section; see [`OrganizationCursor`] for parent and limit custody.
#[napi]
pub struct OrganizationDetailCursor {
    inner: pale_sdk::OrganizationDetailCursor,
}

/// One typed detail section with its parent, visit count and continuation.
#[napi]
pub struct OrganizationDetailPage {
    inner: pale_sdk::OrganizationDetailPage,
}

/// Query one communication resolution's resources; see [`OrganizationCursor`] for custody.
#[napi]
pub struct OrganizationCommunicationResolutionHandle {
    inner: pale_sdk::OrganizationCommunicationResolutionHandle,
}

/// Query one service assignment's members; see [`OrganizationCursor`] for custody.
#[napi]
pub struct OrganizationServiceAssignmentHandle {
    inner: pale_sdk::OrganizationServiceAssignmentHandle,
}

/// Query one responsibility projection's members; see [`OrganizationCursor`] for custody.
#[napi]
pub struct OrganizationResponsibilityProjectionHandle {
    inner: pale_sdk::OrganizationResponsibilityProjectionHandle,
}

/// Query one genealogy link's members; see [`OrganizationCursor`] for custody.
#[napi]
pub struct OrganizationGenealogyHandle {
    inner: pale_sdk::OrganizationGenealogyHandle,
}

#[napi]
impl PortableWorkspaceCompilation {
    /// Page directional services at an exact provider or consumer scope.
    /// Retains full contract direction, requirements and source/detail handles.
    /// Empty pages may continue; this does not prove scope existence or native readiness.
    /// # Errors
    /// Uses the same limit, cursor and diagnostic rules as [`Self::inspect_organization_endpoint`].
    #[napi]
    pub fn inspect_organization_incident_services(
        &self,
        env: Env,
        scope: OrganizationScopeRef,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationCursor>,
    ) -> Result<OrganizationPage> {
        self.inner
            .inspect_organization_incident_services(
                &scope.into_api(),
                after.map(|value| &value.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationPage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Find an exact endpoint or its binding witnesses in the retained compilation.
    /// Empty partial pages do not prove absence. A continuation binds the original
    /// endpoint, owner and limits. This grants no recipient disclosure permission.
    /// # Errors
    /// Malformed transport values reject before the SDK. Identifier, limit, custody
    /// and copy-budget failures retain the SDK diagnostic and do not advance a cursor.
    #[napi]
    pub fn inspect_organization_endpoint(
        &self,
        env: Env,
        endpoint: OrganizationNode,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationCursor>,
    ) -> Result<OrganizationPage> {
        self.inner
            .inspect_organization_endpoint(
                &endpoint.into_api(),
                after.map(|value| &value.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationPage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Read one-hop explicit and already-derived relations at either endpoint.
    /// The application preserves direction, intervals and parallel relations.
    /// This is not transitive impact, endpoint-existence proof or native observation.
    /// # Errors
    /// Uses the same query limits and failure rules as [`Self::inspect_organization_endpoint`].
    #[napi]
    pub fn inspect_organization_incident_relations(
        &self,
        env: Env,
        endpoint: OrganizationNode,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationCursor>,
    ) -> Result<OrganizationPage> {
        self.inner
            .inspect_organization_incident_relations(
                &endpoint.into_api(),
                after.map(|value| &value.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationPage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Inspect bounded shallow organization records without another compilation.
    /// Raw limits reach the SDK unchanged; 0 and 257 are not silently defaulted.
    /// Copy budget covers SDK UTF-8 payload only. Keep the original cursor on error.
    /// This inspects caller-supplied input, not authenticated recipient delivery.
    /// # Errors
    /// Nonintegral/out-of-representation limits are `InvalidArg`; SDK query, custody,
    /// section and copy failures preserve the canonical diagnostic.
    #[napi]
    pub fn inspect_organization(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationCursor>,
    ) -> Result<OrganizationPage> {
        self.inner
            .inspect_organization(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
}

#[napi]
impl OrganizationEntry {
    /// Copy the exact selected row payload; modifying it never mutates native custody.
    #[napi(getter)]
    pub fn row(&self) -> OrganizationRow {
        OrganizationRow::from_api(self.inner.row())
    }
    /// Independently retain the exact record for nested/source queries.
    #[napi(getter)]
    pub fn handle(&self) -> OrganizationRecordHandle {
        OrganizationRecordHandle {
            inner: self.inner.handle().clone(),
        }
    }
}

#[napi]
impl OrganizationPage {
    /// Cumulative endpoint lookup, or null for enumeration and incident queries.
    /// Input selection and effective state remain separately typed facts.
    #[napi(getter)]
    pub fn endpoint_lookup(&self) -> Option<OrganizationEndpointLookup> {
        self.inner
            .endpoint_lookup()
            .map(OrganizationEndpointLookup::from_api)
    }

    /// Independent retained entries; no cursor is needed to keep terminal results alive.
    #[napi]
    pub fn entries(&self) -> Vec<OrganizationEntry> {
        self.inner
            .entries()
            .iter()
            .cloned()
            .map(|inner| OrganizationEntry { inner })
            .collect()
    }
    /// Exact producing compilation, never a digest-only comparison.
    #[napi]
    pub fn belongs_to(&self, owner: &PortableWorkspaceCompilation) -> bool {
        self.inner.belongs_to(&owner.inner)
    }
    /// Workspace that owns the compiled organization.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }
    /// Principal-universe digest of the producing compilation.
    #[napi(getter)]
    pub fn principal_universe_digest(&self) -> String {
        self.inner.principal_universe_digest().to_owned()
    }
    /// Semantic IR digest of the producing compilation.
    #[napi(getter)]
    pub fn semantic_ir_digest(&self) -> String {
        self.inner.semantic_ir_digest().to_owned()
    }
    /// Admission-evidence digest retained by the producing compilation.
    #[napi(getter)]
    pub fn admission_evidence_digest(&self) -> String {
        self.inner.admission_evidence_digest().to_owned()
    }
    /// Whether the compilation selected an organization document.
    #[napi(getter)]
    pub fn organization_selected(&self) -> bool {
        self.inner.organization_selected()
    }
    /// Record cells visited to produce this page.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }
    /// UTF-8 bytes charged to this SDK query's copied-string budget.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }
    /// Whether organization enumeration is exhausted.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }
    /// Null only when the SDK established enumeration exhaustion.
    #[napi]
    pub fn continuation(&self) -> Option<OrganizationCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| OrganizationCursor { inner })
    }
}

#[napi]
impl OrganizationRecordHandle {
    /// Exact producing compilation, not equal-value reconstructed source.
    #[napi]
    pub fn belongs_to(&self, owner: &PortableWorkspaceCompilation) -> bool {
        self.inner.belongs_to(&owner.inner)
    }
    /// Actual retained workspace coordinate.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }
    /// Page exact source occurrences and roles; unmapped is not absent source.
    /// Empty named pages can continue. All original limits bind continuation.
    /// # Errors
    /// Numeric coercion rejects before the SDK; SDK diagnostics remain unchanged.
    #[napi]
    pub fn sources(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationSourceCursor>,
    ) -> Result<OrganizationSourcePage> {
        self.inner
            .sources(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationSourcePage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
}

/// Exact source mapping with nullable continuation, distinct from unmapped records.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false,
    use_nullable = true
)]
pub enum OrganizationSourceMapping {
    /// No supported exact named mapping for this record; no source-absence claim.
    UnmappedRecord,
    /// Existing source matches and independent traversal frontier.
    Named {
        /// Compiler-assigned source paths and roles.
        occurrences: Vec<AuthoringOccurrence>,
        /// Source candidates visited, including nonmatches.
        inspected: u16,
        /// Exact retained continuation or null after exhaustion.
        continuation: Option<OrganizationSourceCursor>,
    },
}

#[napi]
impl OrganizationSourcePage {
    /// Actual independently retained record, including unmapped/empty results.
    #[napi(getter)]
    pub fn record(&self) -> OrganizationRecordHandle {
        OrganizationRecordHandle {
            inner: self.inner.record().clone(),
        }
    }
    /// SDK copied UTF-8 payload, excluding repeated getters and allocation overhead.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }
    /// Copy the SDK source result without deriving identity from labels.
    /// # Errors
    /// Preserves existing source projection errors when an index cannot fit Node.
    #[napi(getter)]
    pub fn mapping(&self) -> Result<OrganizationSourceMapping> {
        Ok(match self.inner.mapping() {
            pale_sdk::OrganizationSourceMapping::UnmappedRecord => {
                OrganizationSourceMapping::UnmappedRecord
            }
            pale_sdk::OrganizationSourceMapping::Named {
                occurrences,
                inspected,
                continuation,
            } => OrganizationSourceMapping::Named {
                occurrences: occurrences
                    .iter()
                    .map(AuthoringOccurrence::from_api)
                    .collect::<Result<_>>()?,
                inspected: *inspected,
                continuation: continuation
                    .clone()
                    .map(|inner| OrganizationSourceCursor { inner }),
            },
        })
    }
}

#[napi]
impl OrganizationDetailPage {
    /// Actual typed parent, not a value-based reconstruction of its member.
    #[napi(getter)]
    pub fn parent(&self) -> OrganizationDetailParent {
        OrganizationDetailParent::from_api(self.inner.parent())
    }
    /// Source lookup remains under the root record's existing source owner.
    #[napi(getter)]
    pub fn record(&self) -> OrganizationRecordHandle {
        OrganizationRecordHandle {
            inner: self.inner.parent().record().clone(),
        }
    }
    /// One closed section payload; arrays copy only the current bounded page.
    #[napi(getter)]
    pub fn rows(&self) -> OrganizationDetailRows {
        OrganizationDetailRows::from_api(self.inner.rows())
    }
    /// Actual direct member visits, excluding separately queried nested collections.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }
    /// SDK copied string payload, not all native/JavaScript allocation.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }
    /// This section exhausted, not the complete record or World.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }
    /// Actual section/nested-parent continuation, or null after exhaustion.
    #[napi]
    pub fn continuation(&self) -> Option<OrganizationDetailCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| OrganizationDetailCursor { inner })
    }
}

#[napi]
impl OrganizationRecordHandle {
    /// Page the SDK-owned axis resources section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn axis_resources(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .axis_resources(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned leadership participants section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn leadership_participants(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .leadership_participants(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned leadership scopes section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn leadership_scopes(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .leadership_scopes(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned communication resolutions section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn communication_resolutions(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .communication_resolutions(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned lens standards section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn lens_standards(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .lens_standards(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned lens conditions section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn lens_conditions(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .lens_conditions(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned lens rights section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn lens_rights(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .lens_rights(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned service assignments section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn service_assignments(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .service_assignments(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned service rights section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn service_rights(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .service_rights(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned service controllers section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn service_controllers(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .service_controllers(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned service authorities section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn service_authorities(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .service_authorities(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned native references section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn native_references(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .native_references(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned inspection coordinates section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn inspection_coordinates(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .inspection_coordinates(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned inspection states section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn inspection_states(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .inspection_states(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned responsibility selections section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn responsibility_selections(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .responsibility_selections(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned responsibility residences section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn responsibility_residences(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .responsibility_residences(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned responsibility projections section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn responsibility_projections(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .responsibility_projections(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned responsibility genealogy section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn responsibility_genealogy(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .responsibility_genealogy(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
}

#[napi]
impl OrganizationCommunicationResolutionHandle {
    /// Page the SDK-owned authorized closure section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn authorized_closure(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .authorized_closure(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
}

#[napi]
impl OrganizationServiceAssignmentHandle {
    /// Page the SDK-owned stewards section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn stewards(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .stewards(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned pods section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn pods(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .pods(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
}

#[napi]
impl OrganizationResponsibilityProjectionHandle {
    /// Page the SDK-owned native references section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn native_references(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .native_references(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
}

#[napi]
impl OrganizationGenealogyHandle {
    /// Page the SDK-owned responsibilities section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn responsibilities(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .responsibilities(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned projections section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn projections(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .projections(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
    /// Page the SDK-owned occurrences section of this exact parent.
    /// # Errors
    /// Invalid numeric representations reject before dispatch; SDK limit, custody,
    /// unavailable-section and copied-string errors preserve their diagnostics.
    #[napi]
    pub fn occurrences(
        &self,
        env: Env,
        maximum_inspected: f64,
        maximum_returned: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationDetailCursor>,
    ) -> Result<OrganizationDetailPage> {
        self.inner
            .occurrences(
                after.map(|v| &v.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationDetailPage { inner })
            .map_err(|e| diagnostic_error(&env, e))
    }
}
