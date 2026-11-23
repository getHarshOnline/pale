//! Derive a selected project's effective configuration from a compiled workspace.
//!
//! Use [`PaleApi::compile_effective_profile`] to inspect selected operations,
//! capability bindings, repository access, and policy limits. It reuses the
//! [`crate::PortableWorkspaceCompilation`] and accepts the additional private
//! configuration and Package documents; it does not parse the workspace again.
//! This route can project source guidance without runtime admission. Use
//! [`PaleApi::compile_one_shot`] when the caller also needs a complete captured
//! runtime compilation for repository planning. Neither route observes a live machine.

use std::sync::Arc;

use pale_application::compile::{
    CapturedOneShotCompilationInputs, CapturedPackageDocument, CapturedPackageSource, CompileError,
};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire, DiagnosticStageV1,
};

use crate::error::{application_diagnostic_policy, FailureSpecification};
use crate::one_shot::{CompilationDocument, OneShotPrivateInputs};
use crate::{ApiError, PaleApi, PortableWorkspaceCompilation};

mod projection;
mod source_guidance;
use projection::EffectiveProfileProjection;
pub use projection::{
    EffectiveAuthoringInspection, EffectiveProfileCoordinates, EffectiveProfileDeclaredSelection,
    EffectiveProfileDeclaredSelector, EffectiveProfileEffectClass, EffectiveProfileEntry,
    EffectiveProfileEntryDetail, EffectiveProfileExplanation, EffectiveProfileExplanationEvidence,
    EffectiveProfileExplanationHandle, EffectiveProfileFinding, EffectiveProfileModeAxis,
    EffectiveProfileNarrowingReason, EffectiveProfileNativeRealization,
    EffectiveProfileOperationExposure, EffectiveProfilePage, EffectiveProfilePageRequest,
    EffectiveProfileParticipant, EffectiveProfilePathReason, EffectiveProfilePolicyBlock,
    EffectiveProfileProviderBindingRef, EffectiveProfileProviderDisposition,
    EffectiveProfileProviderKind, EffectiveProfileRepositoryAccess,
    EffectiveProfileRequirementDisposition, EffectiveProfileResourceKind,
    EffectiveProfileResponsibility, EffectiveProfileSection, EffectiveProfileSelection,
    InvalidEffectiveProfileExplanationHandle,
};
pub use source_guidance::{SourceGuidanceFile, SourceGuidanceHost, SourceGuidanceProjection};

/// Owned source families required after portable workspace admission.
///
/// The selectors name declarations in the already-admitted workspace. The
/// machine record is admitted as private configuration, but no path,
/// executable, provider, process, or native state is observed by this request.
pub struct EffectiveProfileCompilationRequest {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private_inputs: OneShotPrivateInputs,
    package_documents: Vec<CompilationDocument>,
}

impl std::fmt::Debug for EffectiveProfileCompilationRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EffectiveProfileCompilationRequest")
            .field("project_id_byte_len", &self.project_id.len())
            .field(
                "execution_workspace_id_byte_len",
                &self.execution_workspace_id.len(),
            )
            .field("machine_id_byte_len", &self.machine_id.len())
            .field("private_inputs", &self.private_inputs)
            .field("package_document_count", &self.package_documents.len())
            .finish()
    }
}

impl EffectiveProfileCompilationRequest {
    /// Bind explicit selectors to bounded private and immutable Package bytes.
    ///
    /// Construction performs no parsing, trust transition, machine
    /// observation, activation, or effect. The complete request is admitted by
    /// [`PaleApi::compile_effective_profile`].
    #[must_use]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        private_inputs: OneShotPrivateInputs,
        package_documents: Vec<CompilationDocument>,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private_inputs,
            package_documents,
        }
    }

    pub(crate) fn into_application(
        self,
    ) -> Result<CapturedOneShotCompilationInputs, pale_application::compile::CompileError> {
        let Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private_inputs,
            package_documents,
        } = self;
        let private = private_inputs.into_application()?;
        let documents = package_documents
            .into_iter()
            .map(CompilationDocument::into_parts)
            .map(|(label, bytes)| CapturedPackageDocument::new(label, bytes))
            .collect::<Result<Vec<_>, _>>()?;
        let packages = CapturedPackageSource::new(documents)?;
        Ok(CapturedOneShotCompilationInputs::new(
            project_id,
            execution_workspace_id,
            machine_id,
            private,
            packages,
        ))
    }
}

/// Opaque admitted profile phase retained for bounded inspection.
///
/// The handle contains no source bytes and exposes no compiler IR. It proves
/// portable profile, declared private projection, Package semantics, and
/// composition admission only. Machine/native realization remains unobserved.
#[derive(Clone)]
pub struct EffectiveProfileCompilation {
    pub(crate) compilation: Arc<pale_application::compile::EffectiveProfileCompilation>,
    projection: Arc<EffectiveProfileProjection>,
    pub(crate) diagnostic_identity: pale_core::records::RecordSchemaIdentity,
}

impl std::fmt::Debug for EffectiveProfileCompilation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EffectiveProfileCompilation")
            .field("workspace_id", &self.workspace_id())
            .field("project_id", &self.project_id())
            .field("execution_workspace_id", &self.execution_workspace_id())
            .field("profile_digest", &self.profile_digest())
            .finish()
    }
}

/// Resume one authoring-to-profile query on the same retained compilation.
///
/// This in-process token retains the shared profile projection, full authored
/// identity, requested limit and next row. It cannot be constructed from public
/// coordinates or serialized. Cloning it shares the projection; dropping its
/// last owner releases that reference. It grants no authority or native effect.
#[derive(Clone)]
pub struct EffectiveAuthoringContinuation {
    projection: Arc<EffectiveProfileProjection>,
    identity: crate::AuthoringIdentity,
    limit: u16,
    offset: usize,
}

impl std::fmt::Debug for EffectiveAuthoringContinuation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EffectiveAuthoringContinuation")
            .finish_non_exhaustive()
    }
}

impl PartialEq for EffectiveAuthoringContinuation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.projection, &other.projection)
            && self.identity == other.identity
            && self.limit == other.limit
            && self.offset == other.offset
    }
}

impl Eq for EffectiveAuthoringContinuation {}

/// One bounded row window associated with an authored identity.
///
/// Entries retain the existing profile row and explanation types. The
/// continuation resumes the same immutable query; it does not recompute
/// configuration or imply machine readiness. Each page copies at most 256
/// rows. A row can contain multiple members, so this is not a byte limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveAuthoringPage {
    coordinates: EffectiveProfileCoordinates,
    entries: Vec<EffectiveProfileEntry>,
    continuation: Option<EffectiveAuthoringContinuation>,
}

impl EffectiveAuthoringPage {
    /// Profile identity shared by every row in this page.
    #[must_use]
    pub const fn coordinates(&self) -> &EffectiveProfileCoordinates {
        &self.coordinates
    }

    /// Retained profile rows in deterministic section and row order.
    #[must_use]
    pub fn entries(&self) -> &[EffectiveProfileEntry] {
        &self.entries
    }

    /// Token for the next page, or `None` when the association is complete.
    #[must_use]
    pub const fn continuation(&self) -> Option<&EffectiveAuthoringContinuation> {
        self.continuation.as_ref()
    }
}

impl EffectiveProfileCompilation {
    pub(crate) fn observation_page(
        &self,
        query: &pale_application::observation::ObservationProfileQuery<'_>,
        position: (usize, usize),
    ) -> (
        Vec<EffectiveProfileEntry>,
        Option<(usize, usize)>,
        u16,
        pale_application::observation::ObservationProfileMapping,
    ) {
        self.projection.observation_page(query, position)
    }

    pub(crate) fn from_application(
        compilation: Arc<pale_application::compile::EffectiveProfileCompilation>,
        diagnostic_identity: pale_core::records::RecordSchemaIdentity,
    ) -> Self {
        let projection = Arc::new(EffectiveProfileProjection::from_compilation(&compilation));
        Self {
            compilation,
            projection,
            diagnostic_identity,
        }
    }

    /// Principal workspace selected by the effective profile.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.compilation.profile().workspace_id.as_str()
    }

    /// Canonical logical project selected for inspection.
    #[must_use]
    pub fn project_id(&self) -> &str {
        self.compilation.profile().project_id.as_str()
    }

    /// Canonical execution workspace selected for inspection.
    #[must_use]
    pub fn execution_workspace_id(&self) -> &str {
        self.compilation.profile().execution_workspace_id.as_str()
    }

    /// Canonical identity of the complete portable profile.
    #[must_use]
    pub fn profile_digest(&self) -> &str {
        self.compilation.profile().profile_digest.as_str()
    }

    /// Canonical identity of the private machine-resolved instance.
    ///
    /// This digest binds the admitted private projection used to construct the
    /// profile phase. It is not evidence that the machine, provider, or native
    /// runtime was observed or is ready; those states remain separate
    /// observation axes.
    #[must_use]
    pub fn instance_digest(&self) -> &str {
        self.compilation.instance().instance_digest.as_str()
    }

    /// Identity of the selected and admitted Package semantic closure.
    #[must_use]
    pub fn package_semantic_ir_digest(&self) -> &str {
        self.compilation
            .profile()
            .package_semantic_ir_digest
            .as_str()
    }

    /// Identity of the neutral composition proof retained by this phase.
    #[must_use]
    pub fn composition_plan_digest(&self) -> &str {
        self.compilation
            .experience_composition_plan()
            .plan_digest
            .as_str()
    }

    /// Exact coordinates shared by every bounded inspection page.
    #[must_use]
    pub fn coordinates(&self) -> EffectiveProfileCoordinates {
        self.projection.coordinates()
    }

    /// Project one bounded page from the owned inspection retained at compile time.
    ///
    /// This method reveals compiler-owned members and typed differences already
    /// established by the application-owned effective-profile set model. It
    /// does not reparse source, recompute closure, score a configuration, or
    /// construct machine realization. A zero `limit` selects 64
    /// entries; larger values are capped at 256.
    #[must_use]
    pub fn inspect(&self, request: EffectiveProfilePageRequest) -> EffectiveProfilePage {
        self.projection.page(request)
    }

    /// Return one bounded explanation retained with this exact compilation.
    ///
    /// The opaque handle is meaningful only within the profile coordinates that
    /// produced it. This lookup reparses no source and recomputes no semantic
    /// closure, provider disposition, composition proof, or native state.
    #[must_use]
    pub fn explain(
        &self,
        handle: &EffectiveProfileExplanationHandle,
    ) -> Option<EffectiveProfileExplanation> {
        self.projection.explain(handle)
    }

    /// Join one compiler-resolved authoring identity to retained profile facts.
    ///
    /// Identities can come from [`Self::navigate_authoring`] or
    /// [`PortableWorkspaceCompilation::navigate_authoring`]. The identity's
    /// kind and value select rows from this profile; the result carries this
    /// profile's coordinates, not the source compilation's coordinates. An
    /// unmatched identity returns no rows.
    ///
    /// The join reads the retained projection. It does not parse editor text,
    /// repeat semantic resolution, or infer machine realization. Results are
    /// capped at 64 rows and report truncation.
    #[must_use]
    pub fn inspect_effective_authoring(
        &self,
        identity: &crate::AuthoringIdentity,
    ) -> EffectiveAuthoringInspection {
        self.projection.authoring(identity.kind(), identity.value())
    }

    /// Enumerate profile rows associated with one compiler-resolved identity.
    ///
    /// This uses the same kind-and-value association as
    /// [`Self::inspect_effective_authoring`]. It reads the retained index and
    /// copies only the requested window; it does not scan preceding pages or
    /// recompile source. Zero selects 64 rows; larger limits are capped at 256.
    ///
    /// A continuation binds this projection allocation, the full `identity`
    /// and the raw requested `limit`. A clone of this compilation can resume
    /// it. A fresh compilation cannot, even when its public coordinates match.
    /// Keep the limit unchanged when resuming: `0` and `64` have the same page
    /// size but are distinct requests. An unmatched identity yields a complete
    /// empty page. Dropping a page or token has no configuration effect.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the continuation belongs to another retained
    /// projection, authoring identity or requested limit. The diagnostic does
    /// not disclose the mismatched identities.
    pub fn inspect_effective_authoring_page(
        &self,
        identity: &crate::AuthoringIdentity,
        continuation: Option<&EffectiveAuthoringContinuation>,
        limit: u16,
    ) -> Result<EffectiveAuthoringPage, ApiError> {
        if continuation.is_some_and(|token| {
            !Arc::ptr_eq(&self.projection, &token.projection)
                || token.identity != *identity
                || token.limit != limit
        }) {
            return Err(crate::authoring::authoring_continuation_failure(
                &self.diagnostic_identity,
            ));
        }
        let offset = continuation.map_or(0, |token| token.offset);
        let (entries, next_offset) =
            self.projection
                .authoring_page(identity.kind(), identity.value(), offset, limit);
        Ok(EffectiveAuthoringPage {
            coordinates: self.projection.coordinates(),
            entries,
            continuation: next_offset.map(|offset| EffectiveAuthoringContinuation {
                projection: Arc::clone(&self.projection),
                identity: identity.clone(),
                limit,
                offset,
            }),
        })
    }
}

impl PaleApi {
    /// Derive the effective configuration for the project and machine in `request`.
    ///
    /// Use the result to inspect the selected operations, capability bindings,
    /// repository access, and policy limits. `workspace` must already have
    /// compiled successfully; its parsing and admission are not repeated.
    /// The request supplies private configuration and Package documents for the
    /// selected project, execution workspace, and machine.
    ///
    /// The resulting handle shares the admitted workspace and stops before
    /// machine observation, rendering, resident admission, activation,
    /// installation, publication, or native effect. A configured provider is
    /// not evidence that the provider is available.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the selected topology, private records,
    /// distribution lock, Package graph, profile, semantic projection, or
    /// composition proof is inconsistent.
    pub fn compile_effective_profile(
        &self,
        workspace: &PortableWorkspaceCompilation,
        request: EffectiveProfileCompilationRequest,
    ) -> Result<EffectiveProfileCompilation, ApiError> {
        let context = EffectiveProfileFailureContext {
            project_id: request.project_id.clone(),
            execution_workspace_id: request.execution_workspace_id.clone(),
            machine_id: request.machine_id.clone(),
        };
        let inputs = request.into_application().map_err(|error| {
            effective_profile_failure(&self.diagnostic_identity, error, &context)
        })?;
        let compilation = pale_application::compile::compile_captured_effective_profile_inputs(
            inputs,
            workspace.application_universe(),
        )
        .map_err(|error| effective_profile_failure(&self.diagnostic_identity, error, &context))?;
        Ok(EffectiveProfileCompilation::from_application(
            Arc::new(compilation),
            self.diagnostic_identity.clone(),
        ))
    }
}

struct EffectiveProfileFailureContext {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
}

/// Add effective-profile request context without recreating compiler policy.
fn effective_profile_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: CompileError,
    context: &EffectiveProfileFailureContext,
) -> ApiError {
    let (disposition, retry) = application_diagnostic_policy(error.failure_disposition());
    let (subject, mut facts) = match &error {
        CompileError::CompositionInput(source)
            if disposition == DiagnosticDispositionV1::InvalidInput =>
        {
            crate::error::composition_failure_details(source)
        }
        _ => (None, Vec::new()),
    };
    facts.extend([
        fact("project_id", &context.project_id),
        fact("execution_workspace_id", &context.execution_workspace_id),
        fact("machine_id", &context.machine_id),
    ]);
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "effective_profile_compilation_rejected",
            stage: DiagnosticStageV1::Compilation,
            disposition,
            summary: "effective-profile compilation could not establish an admitted profile",
            subject,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: vec![
                "The request stopped while deriving the selected portable profile from admitted workspace authority.",
            ],
            help: Vec::new(),
            facts,
        },
    )
}

fn fact(key: &str, value: &str) -> DiagnosticFactV1Wire {
    DiagnosticFactV1Wire {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}
