//! Node-native handles for API-owned effective-profile inspection.
//!
//! [`pale_sdk::EffectiveProfileCompilation`] owns the admitted profile and
//! page algebra. This leaf retains that opaque handle and forwards its exact
//! canonical page bytes; it does not reconstruct selections or findings.

use napi::bindgen_prelude::{Buffer, ClassInstance};
use napi::{Env, Error, Result, Status};
use napi_derive::napi;
use pale_sdk::{
    EffectiveProfileCompilation as ApiEffectiveProfileCompilation,
    EffectiveProfileCompilationRequest as ApiEffectiveProfileCompilationRequest,
    EffectiveProfileExplanationHandle, EffectiveProfilePageRequest,
    EffectiveProfileSection as ApiEffectiveProfileSection,
};

use crate::error::diagnostic_error;
use crate::{
    authoring::{authoring_page_limit, instance_path, occurrence_request, structural_path},
    AuthoringIdentity, AuthoringInspection, AuthoringInstancePathSegment, AuthoringNavigation,
    AuthoringOccurrencePage, AuthoringOccurrencePageCursor, AuthoringPathSegment,
    CompilationDocument, EffectiveProfileEntry, EffectiveProfileExplanation, OneShotPrivateInputs,
    Pale, PortableWorkspaceCompilation,
};

/// Complete captured input required after portable workspace admission.
#[napi(
    object,
    object_to_js = false,
    js_name = "EffectiveProfileCompilationRequest"
)]
pub struct EffectiveProfileCompilationRequest {
    /// Canonical project selector candidate.
    pub project_id: String,
    /// Canonical execution-workspace selector candidate.
    pub execution_workspace_id: String,
    /// Canonical private machine-binding selector candidate.
    pub machine_id: String,
    /// Fixed-role private inputs captured by the caller.
    pub private_inputs: OneShotPrivateInputs,
    /// Complete immutable Package document batch.
    pub package_documents: Vec<CompilationDocument>,
}

impl EffectiveProfileCompilationRequest {
    /// Consume the host values into the one API-owned request.
    fn into_api(self) -> ApiEffectiveProfileCompilationRequest {
        ApiEffectiveProfileCompilationRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api(),
            self.package_documents
                .into_iter()
                .map(CompilationDocument::into_api)
                .collect(),
        )
    }
}

/// Opaque admitted profile phase; garbage collection has no semantic effect.
#[napi(js_name = "EffectiveProfileCompilation")]
pub struct EffectiveProfileCompilation {
    pub(crate) inner: ApiEffectiveProfileCompilation,
}

#[napi]
impl EffectiveProfileCompilation {
    /// Principal workspace owning the effective profile.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }

    /// Selected logical project.
    #[napi(getter)]
    pub fn project_id(&self) -> String {
        self.inner.project_id().to_owned()
    }

    /// Selected logical execution workspace.
    #[napi(getter)]
    pub fn execution_workspace_id(&self) -> String {
        self.inner.execution_workspace_id().to_owned()
    }

    /// Canonical identity of the complete portable profile.
    #[napi(getter)]
    pub fn profile_digest(&self) -> String {
        self.inner.profile_digest().to_owned()
    }

    /// Canonical identity of the admitted private machine projection.
    ///
    /// This is a compilation coordinate, not machine-readiness evidence.
    #[napi(getter)]
    pub fn instance_digest(&self) -> String {
        self.inner.instance_digest().to_owned()
    }

    /// Identity of the admitted Package semantic closure.
    #[napi(getter)]
    pub fn package_semantic_ir_digest(&self) -> String {
        self.inner.package_semantic_ir_digest().to_owned()
    }

    /// Identity of the neutral composition proof.
    #[napi(getter)]
    pub fn composition_plan_digest(&self) -> String {
        self.inner.composition_plan_digest().to_owned()
    }

    /// Return the exact semantic coordinate batch shared by every page.
    #[napi]
    pub fn coordinates(&self) -> EffectiveProfileCoordinates {
        EffectiveProfileCoordinates::from_api(self.inner.coordinates())
    }

    /// Project one bounded page without reparsing or recompiling.
    #[napi]
    pub fn inspect(
        &self,
        section: EffectiveProfileSection,
        offset: u32,
        limit: u16,
    ) -> EffectiveProfilePage {
        let page = self.inner.inspect(EffectiveProfilePageRequest::new(
            section.into(),
            offset,
            limit,
        ));
        let entry_count = u32::from(page.entry_count());
        EffectiveProfilePage {
            section: page.section().into(),
            offset: page.offset(),
            entry_count,
            entries: page
                .entries()
                .iter()
                .map(EffectiveProfileEntry::from_api)
                .collect(),
            next_offset: page.next_offset(),
            canonical_bytes: page.canonical_bytes().to_vec().into(),
        }
    }

    /// Explain one exact profile row retained by this compilation.
    ///
    /// `None` means the well-formed handle belongs to another profile
    /// coordinate; it is not converted into a fabricated current result.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error unless `handle` is one lowercase
    /// hexadecimal SHA-256 representation.
    #[napi]
    pub fn explain(&self, handle: String) -> Result<Option<EffectiveProfileExplanation>> {
        let handle = EffectiveProfileExplanationHandle::parse(handle)
            .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
        Ok(self
            .inner
            .explain(&handle)
            .map(EffectiveProfileExplanation::from_api))
    }

    /// Inspect generated grammar facts retained by this effective-profile phase.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` carrying the API-owned canonical diagnostic for
    /// an oversized path or incoherent generated authoring metadata.
    #[napi]
    pub fn inspect_authoring(
        &self,
        env: Env,
        source_label: String,
        path: Vec<ClassInstance<AuthoringPathSegment>>,
    ) -> Result<AuthoringInspection> {
        self.inner
            .inspect_authoring(pale_sdk::AuthoringInspectionRequest::new(
                source_label,
                structural_path(&path),
            ))
            .map(AuthoringInspection::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Resolve one source occurrence retained by this effective-profile phase.
    #[napi]
    pub fn navigate_authoring(
        &self,
        source_label: String,
        path: Vec<ClassInstance<AuthoringInstancePathSegment>>,
    ) -> AuthoringNavigation {
        AuthoringNavigation {
            inner: self
                .inner
                .navigate_authoring(pale_sdk::AuthoringNavigationRequest::new(
                    source_label,
                    instance_path(&path),
                )),
        }
    }

    /// Join one compiler-resolved identity to already retained profile rows.
    ///
    /// The identity's kind and value select this profile's rows; the result
    /// carries this profile's coordinates, not an editor-freshness claim.
    #[napi]
    pub fn inspect_effective_authoring(
        &self,
        identity: &AuthoringIdentity,
    ) -> EffectiveAuthoringInspection {
        EffectiveAuthoringInspection::from_api(
            self.inner.inspect_effective_authoring(&identity.inner),
        )
    }

    /// Find occurrences in the retained workspace and private profile sources.
    ///
    /// Limits must be integers from 0 through 65535. Zero selects 64; the SDK
    /// caps each limit at 256. Resume with the same identity and raw limits on
    /// this exact compilation. Results grant no additional disclosure authority.
    ///
    /// # Errors
    ///
    /// Invalid numeric inputs produce `InvalidArg`. An unlike continuation
    /// produces an error carrying the canonical API diagnostic.
    #[napi]
    pub fn authoring_occurrences(
        &self,
        env: Env,
        identity: &AuthoringIdentity,
        maximum_inspected: f64,
        maximum_returned: f64,
        after: Option<&AuthoringOccurrencePageCursor>,
    ) -> Result<AuthoringOccurrencePage> {
        self.inner
            .authoring_occurrences(occurrence_request(
                identity,
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                after,
            ))
            .map(|inner| AuthoringOccurrencePage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Read another page of profile rows associated with an authoring identity.
    ///
    /// The row limit must be an integer from 0 through 65535. Zero selects 64;
    /// the SDK caps it at 256. A continuation requires the same raw limit,
    /// identity, and compilation. This bound does not bound row bytes.
    ///
    /// # Errors
    ///
    /// Invalid numeric inputs produce `InvalidArg`. An unlike continuation
    /// produces an error carrying the canonical API diagnostic.
    #[napi]
    pub fn inspect_effective_authoring_page(
        &self,
        env: Env,
        identity: &AuthoringIdentity,
        limit: f64,
        after: Option<&EffectiveAuthoringContinuation>,
    ) -> Result<EffectiveAuthoringPage> {
        self.inner
            .inspect_effective_authoring_page(
                &identity.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(limit)?,
            )
            .map(|inner| EffectiveAuthoringPage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}

/// Opaque association continuation tied to one identity, profile, and raw limit.
///
/// Each continuation shares the original profile. A page with a continuation
/// also retains that profile; a terminal page retains only its copied result.
/// Garbage collection releases these references. There is no text constructor.
#[napi(js_name = "EffectiveAuthoringContinuation")]
pub struct EffectiveAuthoringContinuation {
    inner: pale_sdk::EffectiveAuthoringContinuation,
}

/// Copied profile rows and a continuation over their original retained owner.
#[napi(js_name = "EffectiveAuthoringPage")]
pub struct EffectiveAuthoringPage {
    inner: pale_sdk::EffectiveAuthoringPage,
}

#[napi]
impl EffectiveAuthoringPage {
    /// Full profile coordinate, including its private instance identity.
    #[napi(getter)]
    pub fn coordinates(&self) -> EffectiveProfileCoordinates {
        EffectiveProfileCoordinates::from_api(self.inner.coordinates().clone())
    }

    /// Copy associated rows using the existing typed profile projection.
    #[napi(getter)]
    pub fn entries(&self) -> Vec<EffectiveProfileEntry> {
        self.inner
            .entries()
            .iter()
            .map(EffectiveProfileEntry::from_api)
            .collect()
    }

    /// Share the continuation, or return `null` when no rows remain.
    #[napi]
    pub fn continuation(&self) -> Option<EffectiveAuthoringContinuation> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| EffectiveAuthoringContinuation { inner })
    }
}

/// Bounded join from one authoring identity to retained profile rows.
#[napi(
    object,
    object_from_js = false,
    js_name = "EffectiveAuthoringInspection"
)]
pub struct EffectiveAuthoringInspection {
    /// Exact admitted profile coordinate owning every associated row.
    pub coordinates: EffectiveProfileCoordinates,
    /// Complete copied rows in the SDK's deterministic association order.
    pub entries: Vec<EffectiveProfileEntry>,
    /// Whether the fixed association ceiling omitted later rows.
    pub truncated: bool,
}

impl EffectiveAuthoringInspection {
    fn from_api(value: pale_sdk::EffectiveAuthoringInspection) -> Self {
        Self {
            coordinates: EffectiveProfileCoordinates::from_api(value.coordinates().clone()),
            entries: value
                .entries()
                .iter()
                .map(EffectiveProfileEntry::from_api)
                .collect(),
            truncated: value.truncated(),
        }
    }
}

/// Closed Profile Explorer section.
#[napi]
#[derive(Clone, Copy)]
pub enum EffectiveProfileSection {
    /// Declared responsibilities and selected path evidence.
    IntendedUseCase,
    /// Effective releases, policy, repositories, and semantic selections.
    EffectiveConfiguration,
    /// Operations and independently typed capability requirements.
    CapabilitiesAndOperations,
    /// Separately qualified owner-native realization.
    MachineRealization,
    /// Exact gaps, narrowing, and policy findings.
    GapsAndExplanations,
}

impl From<EffectiveProfileSection> for ApiEffectiveProfileSection {
    fn from(value: EffectiveProfileSection) -> Self {
        match value {
            EffectiveProfileSection::IntendedUseCase => Self::IntendedUseCase,
            EffectiveProfileSection::EffectiveConfiguration => Self::EffectiveConfiguration,
            EffectiveProfileSection::CapabilitiesAndOperations => Self::CapabilitiesAndOperations,
            EffectiveProfileSection::MachineRealization => Self::MachineRealization,
            EffectiveProfileSection::GapsAndExplanations => Self::GapsAndExplanations,
        }
    }
}

impl From<ApiEffectiveProfileSection> for EffectiveProfileSection {
    fn from(value: ApiEffectiveProfileSection) -> Self {
        match value {
            ApiEffectiveProfileSection::IntendedUseCase => Self::IntendedUseCase,
            ApiEffectiveProfileSection::EffectiveConfiguration => Self::EffectiveConfiguration,
            ApiEffectiveProfileSection::CapabilitiesAndOperations => {
                Self::CapabilitiesAndOperations
            }
            ApiEffectiveProfileSection::MachineRealization => Self::MachineRealization,
            ApiEffectiveProfileSection::GapsAndExplanations => Self::GapsAndExplanations,
        }
    }
}

/// Exact semantic coordinates for one admitted profile inspection.
#[napi(
    object,
    object_from_js = false,
    js_name = "EffectiveProfileCoordinates"
)]
pub struct EffectiveProfileCoordinates {
    /// Principal workspace identity.
    pub workspace_id: String,
    /// Logical project identity.
    pub project_id: String,
    /// Logical execution-workspace identity.
    pub execution_workspace_id: String,
    /// Selected private machine-binding identity.
    pub machine_id: String,
    /// Canonical effective-profile identity.
    pub profile_digest: String,
    /// Canonical admitted private machine-projection identity.
    pub instance_digest: String,
    /// Identity of the exact project-relative desired topology selection.
    pub selected_topology_digest: String,
    /// Admitted Package semantic-IR identity.
    pub package_semantic_ir_digest: String,
    /// Neutral composition-proof identity.
    pub composition_plan_digest: String,
    /// Admitted workspace semantic-IR identity.
    pub workspace_semantic_ir_digest: String,
    /// Owner-proof set identity used for workspace admission.
    pub workspace_admission_evidence_digest: String,
}

impl EffectiveProfileCoordinates {
    pub(crate) fn from_api(coordinates: pale_sdk::EffectiveProfileCoordinates) -> Self {
        Self {
            workspace_id: coordinates.workspace_id,
            project_id: coordinates.project_id,
            execution_workspace_id: coordinates.execution_workspace_id,
            machine_id: coordinates.machine_id,
            profile_digest: coordinates.profile_digest,
            instance_digest: coordinates.instance_digest,
            selected_topology_digest: coordinates.selected_topology_digest,
            package_semantic_ir_digest: coordinates.package_semantic_ir_digest,
            composition_plan_digest: coordinates.composition_plan_digest,
            workspace_semantic_ir_digest: coordinates.workspace_semantic_ir_digest,
            workspace_admission_evidence_digest: coordinates.workspace_admission_evidence_digest,
        }
    }
}

/// One exact API-owned bounded page.
#[napi(object, object_from_js = false, js_name = "EffectiveProfilePage")]
pub struct EffectiveProfilePage {
    /// Section represented by the page.
    pub section: EffectiveProfileSection,
    /// Exact zero-based offset represented by the page.
    pub offset: u32,
    /// Number of complete entries in the page.
    pub entry_count: u32,
    /// Complete copied rows in the exact order of the canonical page.
    pub entries: Vec<EffectiveProfileEntry>,
    /// Next zero-based offset, or `None` for the terminal page.
    pub next_offset: Option<u32>,
    /// Exact RFC 8785 bytes produced once by `pale-sdk`.
    pub canonical_bytes: Buffer,
}

#[napi]
impl Pale {
    /// Compile one selected effective profile over an admitted workspace.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` whose message is the canonical diagnostic
    /// summary and whose `diagnostic` property carries exact structured bytes.
    #[napi]
    pub fn compile_effective_profile(
        &self,
        env: Env,
        workspace: &PortableWorkspaceCompilation,
        request: EffectiveProfileCompilationRequest,
    ) -> Result<EffectiveProfileCompilation> {
        self.api
            .compile_effective_profile(&workspace.inner, request.into_api())
            .map(|inner| EffectiveProfileCompilation { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}
