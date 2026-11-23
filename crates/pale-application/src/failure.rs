//! Sort every application failure into one of four outcomes, so API and native
//! callers know what must change before an operation can be tried again.
//!
//! The outcomes are operational, invalid input, blocked and internal. This module
//! builds on the error types of `compile`, `composition`, `hooks`,
//! `operation_admission`, `packages`, `plan`, `render`, `resident`, `semantic` and
//! `workspace`, and `workspace` uses it in return. Detailed error enums retain
//! ownership and causal evidence. This module owns only the smaller disposition
//! shared by API and native projections; it does not assign protocol codes, retry
//! text, process exits, or effect certainty.

use pale_contracts::SchemaError;
use pale_ports::{ExecutableIdentityError, ProcessProbeError, SourceFilesystemError};

use crate::compile::CompileError;
use crate::composition::CompositionError;
use crate::hooks::RepositoryHookRoutingError;
use crate::operation_admission::OperationAdmissionError;
use crate::packages::PackageError;
use crate::plan::PlanError;
use crate::render::RenderError;
use crate::resident::{ResidentHookIngressAdmissionError, ResidentHookTransportUnavailable};
use crate::semantic::SemanticError;
use crate::workspace::WorkspaceError;

/// Stable consequence of one application-owned failure.
///
/// The disposition answers only what must change before the operation can be
/// attempted honestly. An effect executor may establish an additional
/// effect-unknown state; pure application code cannot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationFailureDisposition {
    /// Host I/O, allocation, process, or platform mechanics failed.
    Operational,
    /// Caller-authored bytes, selectors, evidence, or relationships are invalid.
    InvalidInput,
    /// Valid intent cannot proceed until native or lifecycle state changes.
    Blocked,
    /// Pale application state contradicted a closed invariant.
    Internal,
}

impl RepositoryHookRoutingError {
    /// Classify static repository-hook routing at its grammar owner.
    ///
    /// Every variant describes caller-supplied routing that must be corrected;
    /// no resident lookup, carrier operation, or native effect has started.
    #[must_use]
    pub const fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::InvalidGrammar
            | Self::UnknownEvent
            | Self::InvalidWorkspaceId(_)
            | Self::InvalidRepositoryId(_)
            | Self::InvalidProviderBindingId
            | Self::InvalidProviderBindingIdentity(_) => {
                ApplicationFailureDisposition::InvalidInput
            }
        }
    }
}

impl ResidentHookIngressAdmissionError {
    /// Classify one native hook payload before resident lookup.
    ///
    /// Absence, malformed structure, and static-event disagreement are all
    /// correctable native input. They prove no resident or external effect.
    #[must_use]
    pub const fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::Absent | Self::Malformed(_) | Self::EventMismatch { .. } => {
                ApplicationFailureDisposition::InvalidInput
            }
        }
    }
}

impl ResidentHookTransportUnavailable {
    /// Classify the explicit bootstrap state in which no resident carrier exists.
    #[must_use]
    pub const fn failure_disposition(self) -> ApplicationFailureDisposition {
        ApplicationFailureDisposition::Blocked
    }
}

impl OperationAdmissionError {
    /// Classify a sealed-operation admission failure at its semantic owner.
    ///
    /// The result states only whether authored input, current admitted state,
    /// or a Pale application invariant must change. Protocol codes, retry advice,
    /// effect certainty, and process exits remain responsibilities of their
    /// respective outer projections.
    #[must_use]
    pub const fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::UnknownOperation { .. }
            | Self::InvocationOriginMismatch
            | Self::InvocationPathMismatch
            | Self::MissingToolUseIdentity
            | Self::InvalidToolUseIdentity
            | Self::MissingActivityIdentity
            | Self::HostIdentity(_)
            | Self::MalformedAuthorization
            | Self::UnknownAuthorization { .. }
            | Self::ToolInvocationMismatch => ApplicationFailureDisposition::InvalidInput,
            Self::Unavailable { .. }
            | Self::StaleLease
            | Self::CrossTurnLease
            | Self::ToolInvocationReplay
            | Self::ToolInvocationNotAdmitted
            | Self::ToolOutsideContract { .. }
            | Self::ToolAuthorizationRequired { .. }
            | Self::OperationHalted { .. }
            | Self::OperationSteering { .. }
            | Self::OperationRetrying { .. }
            | Self::OperationEscalated { .. }
            | Self::FailureObservationUnavailable { .. } => ApplicationFailureDisposition::Blocked,
            Self::ArtifactDigestMismatch { .. }
            | Self::MalformedArtifact { .. }
            | Self::SemanticClosureMismatch { .. }
            | Self::InvalidOperationRecord { .. }
            | Self::DuplicateIdentity { .. }
            | Self::BindingCatalogMismatch
            | Self::RouteContextMismatch
            | Self::EntryContractMismatch { .. }
            | Self::AmbiguousToolContract { .. }
            | Self::FailureTransitionMissing
            | Self::Canon(_)
            | Self::SessionState(_) => ApplicationFailureDisposition::Internal,
        }
    }
}

impl CompositionError {
    /// Classify a cause encountered while admitting or linking declarations.
    ///
    /// Caller-selected schema references may be invalid input. Fixed embedded
    /// schema failures and canonicalization remain internal. Once linking has
    /// succeeded, the compiler's output boundary classifies every seal failure
    /// as internal instead of consulting this input-boundary classification.
    #[must_use]
    pub const fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::Schema(_) | Self::Canonicalization(_) => ApplicationFailureDisposition::Internal,
            Self::DeclaredSchema { source, .. } => match source {
                SchemaError::UnknownId(_)
                | SchemaError::IdentityMismatch(_)
                | SchemaError::Invalid(_) => ApplicationFailureDisposition::InvalidInput,
                SchemaError::UnknownDefinition { .. }
                | SchemaError::Encode(_)
                | SchemaError::Parse(_)
                | SchemaError::Compile(_)
                | SchemaError::Integrity(_) => ApplicationFailureDisposition::Internal,
            },
            Self::DuplicateId { .. }
            | Self::MissingReference { .. }
            | Self::ProviderCompatibility { .. }
            | Self::ProviderBinding { .. }
            | Self::Delegation { .. }
            | Self::Edge { .. }
            | Self::Path { .. }
            | Self::DeclarationLimit { .. }
            | Self::PathSelection { .. } => ApplicationFailureDisposition::InvalidInput,
        }
    }
}

impl CompileError {
    /// Classify this failure at the complete compiler trust boundary.
    ///
    /// Planning and rendering failures generally indicate an internal invariant
    /// failure. A host name collision is an authoring conflict between selected
    /// resources, even when each resource was admitted individually.
    #[must_use]
    pub fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::CapturedInput(_) | Self::MissingExecutionPaths { .. } => {
                ApplicationFailureDisposition::InvalidInput
            }
            Self::CompositionInput(source) => source.failure_disposition(),
            Self::SchemaRegistry(_) => ApplicationFailureDisposition::Internal,
            Self::SchemaRegistryMismatch { .. }
            | Self::CompilerIdentityMismatch { .. }
            | Self::LifecycleConflict { .. }
            | Self::TerminalProvider { .. } => ApplicationFailureDisposition::InvalidInput,
            Self::Render(source) if matches!(&**source, RenderError::HostNameCollision { .. }) => {
                ApplicationFailureDisposition::InvalidInput
            }
            Self::CompilationInvariant(_)
            | Self::NativeActionRegistry(_)
            | Self::Plan(_)
            | Self::Render(_) => ApplicationFailureDisposition::Internal,
            Self::PackageLoad(source) => source.failure_disposition(),
            Self::Workspace(source) | Self::Verify(source) => source.failure_disposition(),
            Self::Semantic(source) => source.failure_disposition(),
        }
    }
}

impl RenderError {
    /// Classify a direct render-boundary failure.
    #[must_use]
    pub const fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::InvalidInput { .. }
            | Self::InvalidInputSource { .. }
            | Self::HostNameCollision { .. } => ApplicationFailureDisposition::InvalidInput,
            Self::Projection { .. } | Self::ProjectionSource { .. } => {
                ApplicationFailureDisposition::Internal
            }
        }
    }
}

impl SemanticError {
    /// Classify semantic admission without discarding a nested Package cause.
    #[must_use]
    pub fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::Package(source) => source.failure_disposition(),
            Self::Diagnostics(_) => ApplicationFailureDisposition::InvalidInput,
        }
    }
}

impl PackageError {
    /// Classify Package admission at its semantic owner.
    #[must_use]
    pub fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::SchemaCheck(source) => embedded_schema_failure_disposition(source),
            Self::Read { .. } | Self::Allocation { .. } | Self::UnsupportedPlatform { .. } => {
                ApplicationFailureDisposition::Operational
            }
            Self::SourceDecode { .. } => ApplicationFailureDisposition::Internal,
            Self::Diagnostics(_)
            | Self::InvalidManifest { .. }
            | Self::NestedPackageManifest { .. }
            | Self::SourceInventoryMismatch { .. }
            | Self::ReleaseMismatch { .. }
            | Self::MissingLockedPackage { .. }
            | Self::MissingExactDependency { .. }
            | Self::IncompatibleCompiler { .. }
            | Self::ReleaseClosure(_)
            | Self::ResourceDeclarationMismatch { .. }
            | Self::InvalidBodyCustody { .. }
            | Self::InvalidMarkdownLink { .. }
            | Self::InvalidResourceLink { .. }
            | Self::InvalidOperationProcedure { .. }
            | Self::InvalidResourceSelector { .. }
            | Self::GoverningAlgebra(_)
            | Self::TomlSyntax { .. }
            | Self::PageHeader { .. }
            | Self::TomlToJson { .. }
            | Self::SourceSchema { .. }
            | Self::GraphInvariant(_)
            | Self::DuplicatePackage { .. }
            | Self::InvalidEntry { .. }
            | Self::MissingSnapshotFile { .. }
            | Self::NotUtf8 { .. }
            | Self::SnapshotLimit { .. } => ApplicationFailureDisposition::InvalidInput,
        }
    }
}

impl WorkspaceError {
    /// Classify private-workspace admission without flattening native failures.
    #[must_use]
    pub fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::Input(source) => source_filesystem_disposition(source),
            Self::SchemaCheck { source, .. } => embedded_schema_failure_disposition(source),
            Self::NativeHarnessProbe { source, .. }
            | Self::SurfaceExecutableProbe { source, .. } => process_probe_disposition(source),
            Self::SurfaceExecutableIdentity { source, .. } => {
                executable_identity_disposition(source)
            }
            Self::InputChanged(_) => ApplicationFailureDisposition::Operational,
            Self::InputNotUtf8(_)
            | Self::PackagePageHeader { .. }
            | Self::TomlSyntax { .. }
            | Self::Decode { .. }
            | Self::UnrepresentableDerivationSourceId { .. }
            | Self::MissingParent(_)
            | Self::InvalidDesiredTopology(_)
            | Self::UnknownProject(_)
            | Self::UnknownExecutionWorkspace(_)
            | Self::ProjectExecutionWorkspaceMembership { .. }
            | Self::MissingPackageVersion(_)
            | Self::DuplicateLockedPackage { .. }
            | Self::MissingRunPolicy(_)
            | Self::DuplicateRunPolicy(_)
            | Self::InvalidRunPolicy { .. }
            | Self::InvalidLifecycleBehavior { .. }
            | Self::DuplicateLifecycleBehavior(_)
            | Self::UnresolvedRepository { .. }
            | Self::UnresolvedAuthorityRoot { .. }
            | Self::UnknownFamily { .. }
            | Self::DuplicateRepository(_)
            | Self::DuplicateProviderRepositoryProjection { .. }
            | Self::UnknownRepositoryCommandRole(_)
            | Self::ConflictingRepositoryCommand { .. }
            | Self::NonAbsolutePath { .. }
            | Self::NonDistinctPath(_)
            | Self::EscapedReference(_)
            | Self::DuplicateCapturedSource(_)
            | Self::MissingCapturedSourceRoot(_)
            | Self::InvalidCapturedSourceRoot
            | Self::CapturedPrivateInput(_)
            | Self::CapturedDocumentTooLarge { .. }
            | Self::CapturedDocumentCountExceeded { .. }
            | Self::InvalidWorkspaceComponent(_)
            | Self::InvalidWorkspaceModule(_)
            | Self::InvalidWorkspaceLock(_)
            | Self::ReleaseClosure(_)
            | Self::MissingReference { .. }
            | Self::MissingDistributionRoot(_)
            | Self::IdentityMismatch { .. }
            | Self::UnknownActivationRoot { .. }
            | Self::EmptyActivationRoots
            | Self::DuplicateActivationRoot { .. }
            | Self::ActivationAdditionalRootOverlap { .. }
            | Self::UnknownAdditionalRoot { .. }
            | Self::DuplicateAdditionalRoot { .. }
            | Self::SessionUnknownAuthority(_)
            | Self::ReadingOrderNotConfined(_)
            | Self::SessionStateNotAbsolute(_)
            | Self::DurableEvidenceNotAbsolute(_)
            | Self::MaterializationLineageNotAbsolute(_)
            | Self::DuplicateLanguageIdentity { .. }
            | Self::NonCanonicalLanguageSet { .. }
            | Self::WorkspaceDigestMismatch { .. }
            | Self::ExportProjectionMismatch(_)
            | Self::SourceDeliveryUnavailable { .. }
            | Self::ResolutionLockMismatch(_)
            | Self::UniverseLockMismatch(_)
            | Self::WorkspaceGraphLimit { .. }
            | Self::NamespaceResolution(_)
            | Self::VisibilityDenied(_)
            | Self::OnwardPropagationDenied(_)
            | Self::PolicyWidening(_)
            | Self::ImportVisibilityWidening { .. }
            | Self::ImportPolicyWidening { .. }
            | Self::PolicyCombination(_)
            | Self::InadmissibleDeclarationCycle(_)
            | Self::PortableInputLeakage(_)
            | Self::ResponsibilityDerivation(_)
            | Self::OrganizationalTranslation(_)
            | Self::MissingDirectionalService { .. } => ApplicationFailureDisposition::InvalidInput,
            Self::ActivationRequiresSurfaceExecutable(_)
            | Self::ActivationRequiresSessionState(_)
            | Self::ActivationRequiresDurableEvidence(_)
            | Self::ActivationRequiresNativeHarness { .. }
            | Self::InvalidExecutionPathProviderWitness { .. }
            | Self::UnsortedOrDuplicateNativeHarnessBindings
            | Self::NativeHarnessUtf8 { .. }
            | Self::NativeHarnessVersionMismatch { .. }
            | Self::SurfaceExecutableMissing
            | Self::SurfaceExecutableIdentityJson { .. }
            | Self::SurfaceExecutableIdentitySchema { .. }
            | Self::SurfaceExecutableIdentityDecode { .. }
            | Self::SurfaceExecutablePlatformMismatch { .. }
            | Self::SessionStateNotFound(_)
            | Self::SessionStateNotDirectory(_)
            | Self::SessionStateNotConfined(_)
            | Self::SessionStateUnsafePermissions(_)
            | Self::DurableEvidenceNotFound(_)
            | Self::DurableEvidenceNotDirectory(_)
            | Self::DurableEvidenceNotConfined(_)
            | Self::DurableEvidenceUnsafePermissions(_)
            | Self::MaterializationLineageMissing(_)
            | Self::MaterializationLineageNotFound(_)
            | Self::MaterializationLineageNotDirectory(_)
            | Self::MaterializationLineageNotConfined(_)
            | Self::MaterializationLineageUnsafePermissions(_)
            | Self::SurfaceExecutablePathMismatch { .. }
            | Self::SurfaceExecutableVersionMismatch { .. }
            | Self::SurfaceExecutableProtocolMismatch { .. }
            | Self::ReadingOrderMissing(_)
            | Self::ReadingOrderEscapesRoot { .. }
            | Self::SessionPathMissing { .. }
            | Self::SessionPathObservation { .. }
            | Self::SessionPathNotUtf8 { .. }
            | Self::PublisherAdmissionUnavailable { .. }
            | Self::ReleaseStateNotAdmissible { .. }
            | Self::ProtectedDeliveryAdmissionUnavailable { .. } => {
                ApplicationFailureDisposition::Blocked
            }
            Self::TomlToJson { .. }
            | Self::InvalidProfileIdentity { .. }
            | Self::Canon(_)
            | Self::GeneratedRecordSchema(_)
            | Self::InvalidDerivationReceipt(_)
            | Self::DerivationSemanticMismatch
            | Self::WorkspaceSemanticIrDigestMismatch(_)
            | Self::PortableObservationOverflow { .. } => ApplicationFailureDisposition::Internal,
        }
    }
}

impl PlanError {
    /// Classify one direct planning failure at the boundary that supplied it.
    #[must_use]
    pub const fn failure_disposition(&self) -> ApplicationFailureDisposition {
        match self {
            Self::DuplicateBaselineTarget(_)
            | Self::InvalidBaseline(_)
            | Self::InvalidSuppliedPlan(_)
            | Self::InvalidMaterializationState(_)
            | Self::InvalidRunTarget(_)
            | Self::InvalidRunResult(_)
            | Self::InvalidReceipt(_)
            | Self::InvalidLineageTransition { .. } => ApplicationFailureDisposition::InvalidInput,
            Self::RepositoryMutationDenied { .. } | Self::OwnershipTransition(_) => {
                ApplicationFailureDisposition::Blocked
            }
            Self::InvalidCompilation(_)
            | Self::InvalidIdentity { .. }
            | Self::InvalidManifest(_)
            | Self::DuplicateSurfaceTarget(_)
            | Self::InvalidSurface(_)
            | Self::PlanInvariant { .. }
            | Self::Canon(_)
            | Self::PlanSeal(_)
            | Self::ReceiptSeal(_)
            | Self::Schema(_) => ApplicationFailureDisposition::Internal,
        }
    }
}

/// Classify a schema failure after the Pale application selected the descriptor.
///
/// This is deliberately not a context-free policy for every caller-selected
/// schema lookup. At application boundaries the requested descriptor is
/// already fixed: an invalid instance or claimed identity is authored input,
/// while a missing, malformed, or incoherent embedded descriptor contradicts
/// the application's compiled registry.
#[must_use]
pub const fn embedded_schema_failure_disposition(
    error: &SchemaError,
) -> ApplicationFailureDisposition {
    match error {
        SchemaError::Invalid(_) | SchemaError::IdentityMismatch(_) => {
            ApplicationFailureDisposition::InvalidInput
        }
        SchemaError::UnknownId(_)
        | SchemaError::UnknownDefinition { .. }
        | SchemaError::Encode(_)
        | SchemaError::Parse(_)
        | SchemaError::Compile(_)
        | SchemaError::Integrity(_) => ApplicationFailureDisposition::Internal,
    }
}

const fn source_filesystem_disposition(
    error: &SourceFilesystemError,
) -> ApplicationFailureDisposition {
    match error {
        SourceFilesystemError::InvalidEntry { .. } | SourceFilesystemError::Limit { .. } => {
            ApplicationFailureDisposition::InvalidInput
        }
        SourceFilesystemError::Io { .. }
        | SourceFilesystemError::Allocation { .. }
        | SourceFilesystemError::UnsupportedPlatform { .. } => {
            ApplicationFailureDisposition::Operational
        }
    }
}

const fn executable_identity_disposition(
    error: &ExecutableIdentityError,
) -> ApplicationFailureDisposition {
    match error {
        ExecutableIdentityError::NotAbsolute(_) => ApplicationFailureDisposition::InvalidInput,
        ExecutableIdentityError::NotFound(_)
        | ExecutableIdentityError::Symlink(_)
        | ExecutableIdentityError::NotRegular(_)
        | ExecutableIdentityError::NotExecutable(_)
        | ExecutableIdentityError::DigestMismatch { .. } => ApplicationFailureDisposition::Blocked,
        ExecutableIdentityError::IdentityChanged(_)
        | ExecutableIdentityError::UnsupportedPlatform { .. } => {
            ApplicationFailureDisposition::Operational
        }
    }
}

const fn process_probe_disposition(error: &ProcessProbeError) -> ApplicationFailureDisposition {
    match error {
        ProcessProbeError::Executable(source) => executable_identity_disposition(source),
        ProcessProbeError::Spawn(_)
        | ProcessProbeError::Wait(_)
        | ProcessProbeError::Timeout { .. }
        | ProcessProbeError::Capture { .. }
        | ProcessProbeError::Terminate(_) => ApplicationFailureDisposition::Operational,
        ProcessProbeError::OutputTooLarge { .. } | ProcessProbeError::NonZero { .. } => {
            ApplicationFailureDisposition::Blocked
        }
        ProcessProbeError::PipeUnavailable { .. } => ApplicationFailureDisposition::Internal,
    }
}

#[cfg(test)]
#[path = "../tests/internal/failure.rs"]
mod tests;
