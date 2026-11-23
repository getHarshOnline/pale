//! Compiler-selected grammars, identities and source occurrences for authoring.
//!
//! The linker supplies semantic identities and resolved references. SDK and
//! editor clients present these facts without parsing or resolving them again.
//! Captured definitions remain authorable even when they are not selected for
//! the compiled universe; their presence does not establish proof membership.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::ops::Bound;

use super::link::{LinkedWorkspace, QualifiedDeclarationRef};
use super::source::ParsedWorkspaceSource;
use super::WorkspaceError;
use pale_core::paths::MAX_CONFINED_RELATIVE_PATH_BYTES;
use pale_core::records::{
    WorkspaceComponentId, WorkspaceDeclarationV1, WorkspaceDerivationSourceV1,
    WorkspaceDerivationSubjectV1, WorkspaceId, WorkspaceModuleId, WorkspaceResourceId,
};

use super::source_admission::WORKSPACE_FILE_LIMIT;

mod project_context;
mod schema;
pub use schema::{
    inspect_authoring_node, AuthoringFieldFact, AuthoringFieldPresence, AuthoringLiteral,
    AuthoringLiteralFact, AuthoringNodeFact, AuthoringPathSegment, AuthoringSchemaError,
    AuthoringValueKind,
};

/// Exact annotated schema selected for one admitted TOML source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthoringTomlGrammar {
    schema_id: &'static str,
    definition: Option<&'static str>,
}

impl AuthoringTomlGrammar {
    /// Select one complete registered record grammar.
    #[must_use]
    pub const fn record(schema_id: &'static str) -> Self {
        Self {
            schema_id,
            definition: None,
        }
    }

    /// Select one named definition from a registered definition-set grammar.
    #[must_use]
    pub const fn definition(schema_id: &'static str, definition: &'static str) -> Self {
        Self {
            schema_id,
            definition: Some(definition),
        }
    }

    /// Descriptor-registry identity of the selected schema resource.
    #[must_use]
    pub const fn schema_id(self) -> &'static str {
        self.schema_id
    }

    /// Named `$defs` member when the document is admitted against one definition.
    #[must_use]
    pub const fn definition_name(self) -> Option<&'static str> {
        self.definition
    }
}

/// Compiler-selected authoring treatment for one logical source document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoringDocumentKind {
    /// A typed Package declaration followed by its Markdown body.
    PackagePage,
    /// TOML admitted against an exact Rust-owned grammar.
    Toml(AuthoringTomlGrammar),
    /// Markdown or other substantive text that is not Pale Workspace TOML grammar.
    Text,
}

/// Closed compiler source roles available to configuration-authoring clients.
///
/// This is the single role-to-grammar relation. Editor and binding leaves may
/// project their native role vocabulary into this enum, but must not repeat the
/// schema selection or infer it from a filename or document body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoringSourceRole {
    /// Root portable workspace manifest.
    WorkspaceManifest,
    /// Reusable workspace module.
    WorkspaceModule,
    /// Workspace-language primitive declarations.
    WorkspaceLanguagePrimitives,
    /// Responsibility derivation declarations.
    ResponsibilityDerivation,
    /// Organization-to-responsibility translation declarations.
    OrganizationalTranslation,
    /// Experience-composition declarations.
    ExperienceComposition,
    /// Exact module-resolution lock.
    WorkspaceResolutionLock,
    /// Exact workspace-universe lock.
    WorkspaceUniverseLock,
    /// Private capability policy and bindings.
    Capabilities,
    /// Private machine realization.
    MachineBinding,
    /// Exact Package distribution lock.
    DistributionLock,
    /// Private lifecycle prose.
    LifecycleGuidance,
    /// Package manifest.
    PackageManifest,
    /// Package doctrine declarations.
    PackageDoctrine,
    /// Package operation declarations.
    PackageOperations,
    /// Package role declarations.
    PackageRoles,
    /// Package stack declarations.
    PackageStacks,
    /// Package run-mode declarations.
    PackageRunModes,
    /// Package policy-family declaration.
    PackagePolicyFamily,
    /// Package governing-order declaration.
    PackageGoverningOrder,
    /// Package capability-policy declaration.
    PackageCapabilityPolicy,
    /// Package tool-recipe declaration.
    PackageToolRecipe,
    /// A Package page with a TOML header and a Markdown body.
    PackagePage,
    /// A typed Package data document.
    PackageData,
    /// Immutable Package Markdown or documentation.
    PackageText,
}

impl AuthoringSourceRole {
    /// Locate the TOML region for structural help without admitting its content.
    ///
    /// An unfinished page header remains inspectable within the shared size
    /// bound. Missing opening delimiters, oversized headers and text roles
    /// return no region. Coordinates refer to the original UTF-8 source.
    #[must_use]
    pub fn declaration_range(self, source: &str) -> Option<std::ops::Range<usize>> {
        match self.document_kind() {
            AuthoringDocumentKind::PackagePage => crate::packages::page_header_range(source),
            AuthoringDocumentKind::Toml(_) => Some(0..source.len()),
            AuthoringDocumentKind::Text => None,
        }
    }

    /// Return the exact compiler grammar or text custody for this role.
    #[must_use]
    pub const fn document_kind(self) -> AuthoringDocumentKind {
        let grammar = match self {
            Self::WorkspaceManifest => AuthoringTomlGrammar::record("pale-workspace.v1"),
            Self::WorkspaceModule => AuthoringTomlGrammar::record("workspace-module.v1"),
            Self::WorkspaceLanguagePrimitives => {
                AuthoringTomlGrammar::record("workspace-language-primitives.v1")
            }
            Self::ResponsibilityDerivation => {
                AuthoringTomlGrammar::record("responsibility-derivation.v1")
            }
            Self::OrganizationalTranslation => {
                AuthoringTomlGrammar::record("organizational-translation.v1")
            }
            Self::ExperienceComposition => {
                AuthoringTomlGrammar::record("workspace-composition-source.v1")
            }
            Self::WorkspaceResolutionLock => {
                AuthoringTomlGrammar::record("workspace-resolution-lock.v1")
            }
            Self::WorkspaceUniverseLock => {
                AuthoringTomlGrammar::record("workspace-universe-lock.v1")
            }
            Self::Capabilities => AuthoringTomlGrammar::record("capabilities.v1"),
            Self::MachineBinding => AuthoringTomlGrammar::record("machine-binding.v1"),
            Self::DistributionLock => AuthoringTomlGrammar::record("distribution-lock.v1"),
            Self::PackageManifest => AuthoringTomlGrammar::record("agent-surface-package.v1"),
            Self::PackageDoctrine => AuthoringTomlGrammar::record("doctrine-entry.v1"),
            Self::PackageOperations => AuthoringTomlGrammar::record("operation.v1"),
            Self::PackageRoles => AuthoringTomlGrammar::record("role.v1"),
            Self::PackageStacks => AuthoringTomlGrammar::record("stack.v1"),
            Self::PackageRunModes => AuthoringTomlGrammar::record("run-mode.v1"),
            Self::PackagePolicyFamily => AuthoringTomlGrammar::record("policy-family.v1"),
            Self::PackageGoverningOrder => AuthoringTomlGrammar::record("governing-order.v1"),
            Self::PackageCapabilityPolicy => {
                AuthoringTomlGrammar::definition("capability-procedure.v1", "CapabilityPolicy")
            }
            Self::PackageToolRecipe => {
                AuthoringTomlGrammar::definition("capability-procedure.v1", "ToolRecipe")
            }
            Self::PackagePage => return AuthoringDocumentKind::PackagePage,
            Self::PackageData => AuthoringTomlGrammar::record("package-data.v1"),
            Self::LifecycleGuidance | Self::PackageText => return AuthoringDocumentKind::Text,
        };
        AuthoringDocumentKind::Toml(grammar)
    }
}

/// Source-free exact grammar map retained by an admitted compiler phase.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthoringSourceMap {
    documents: BTreeMap<String, AuthoringDocumentKind>,
}

/// One exact step through an admitted TOML instance.
///
/// Unlike [`AuthoringPathSegment`], an [`Index`](Self::Index) preserves which
/// member of a repeated table or array owns a semantic occurrence. The path is
/// source-free: it carries no text, byte range, filesystem path, or editor
/// protocol coordinate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthoringInstancePathSegment {
    /// Descend through one decoded TOML key.
    Field(String),
    /// Select one exact member of an ordered TOML array.
    Index(usize),
}

/// Exact decoded-key and sequence coordinate retained by a compiler owner.
///
/// The structured decoder, schema instance, or typed linker creates the path
/// while original ordering is available. Presentation leaves must not recreate
/// it by parsing a dotted path or JSON Pointer string. This carries no source
/// body or byte range and does not itself prove admission or subject presence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceAuthoringLocation {
    source_label: String,
    path: Vec<AuthoringInstancePathSegment>,
}

impl WorkspaceAuthoringLocation {
    /// Retain a structured coordinate already resolved by its compiler owner.
    pub(super) fn new(source_label: String, path: Vec<AuthoringInstancePathSegment>) -> Self {
        Self { source_label, path }
    }

    /// Borrow the compiler-retained label. Record anchors use manifest-relative
    /// labels; failure coordinates may instead retain a filesystem source path.
    #[must_use]
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Borrow the exact decoded-key and sequence path.
    #[must_use]
    pub fn path(&self) -> &[AuthoringInstancePathSegment] {
        &self.path
    }

    pub(super) fn copied_string_bytes(&self) -> usize {
        location_copied_string_bytes(&self.source_label, &self.path)
    }
}

/// Closed semantic namespaces navigable from workspace-language source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthoringIdentityKind {
    /// One exact compiler-admitted source document.
    SourceDocument,
    /// Principal-owned workspace root.
    Workspace,
    /// Canonical desired project.
    Project,
    /// Canonical desired execution workspace.
    ExecutionWorkspace,
    /// Canonical desired repository.
    Repository,
    /// Neutral execution-path selection.
    ExecutionPath,
    /// Reusable workspace module.
    Module,
    /// Immutable release of one workspace module.
    ModuleRelease,
    /// Content-addressed workspace-language component.
    Component,
    /// Named interface exported by one module.
    Export,
    /// Namespace scoped to one owning module.
    Namespace,
    /// Namespace-qualified declaration scoped to one owning module.
    Declaration,
    /// Logical resource selection scoped to one module.
    Resource,
    /// Private run-policy declaration.
    RunPolicy,
    /// Private lifecycle behavior.
    LifecycleBehavior,
    /// Portable capability identity.
    Capability,
    /// Machine-local authority-root projection.
    AuthorityRoot,
    /// Exact provider-binding projection.
    ProviderBinding,
    /// Private machine-binding identity.
    Machine,
    /// Attributable responsibility-derivation identity.
    ResponsibilityDerivation,
    /// Responsibility native-projection identity.
    ResponsibilityProjection,
    /// Responsibility Genealogy-link identity.
    ResponsibilityGenealogyLink,
    /// Namespace-qualified logical scope.
    LogicalScope,
    /// Namespace-qualified logical organization.
    LogicalOrganization,
    /// Namespace-qualified organizational group.
    OrganizationalGroup,
    /// Namespace-qualified discipline tree.
    DisciplineTree,
    /// Namespace-qualified stable organizational role.
    OrganizationalRole,
    /// Organizational occupant identity introduced by an explicit binding.
    OrganizationalOccupant,
    /// Organizational provider identity introduced by an explicit binding.
    OrganizationalProvider,
    /// Provider-neutral communication-actor binding identity.
    CommunicationActor,
    /// Topic-scoped leadership-participation identity.
    LeadershipTopic,
    /// Organizational relation identity.
    OrganizationalRelation,
    /// Owner-selected organizational object.
    OrganizationalObject,
    /// Group-lens authority-profile identity.
    GroupLensProfile,
    /// Directional-service contract identity.
    DirectionalService,
    /// Organizational native-projection identity.
    OrganizationalProjection,
    /// Organizational inspection identity.
    OrganizationalInspection,
    /// Separately tagged participant in the neutral composition graph.
    CompositionParticipant,
    /// Neutral composition responsibility-contract identity.
    CompositionResponsibilityContract,
    /// Neutral composition participant/provider-manifest identity.
    CompositionParticipantManifest,
    /// Provider declared within a composition manifest.
    CompositionProvider,
    /// Neutral composition provider-binding identity.
    CompositionProviderBinding,
    /// Neutral composition entry-contract identity.
    CompositionEntryContract,
    /// Neutral request-translation identity.
    CompositionRequestTranslation,
    /// Neutral result-translation identity.
    CompositionResultTranslation,
    /// Neutral termination-contract identity.
    CompositionTerminationContract,
    /// Neutral settlement-contract identity.
    CompositionSettlementContract,
    /// Neutral continuity-contract identity.
    CompositionContinuityContract,
    /// Typed neutral composition-edge identity.
    CompositionEdge,
    /// Selected neutral experience-path identity.
    ExperiencePath,
    /// Package-owned doctrine resource.
    PackageDoctrine,
    /// Package-owned operation-archetype resource.
    PackageOperationArchetype,
    /// Package-owned concrete operation resource.
    PackageOperation,
    /// Package-owned role-archetype resource.
    PackageRoleArchetype,
    /// Package-owned specialist-role resource.
    PackageRole,
    /// Package-owned stack resource.
    PackageStack,
    /// Package-owned run-mode resource.
    PackageRunMode,
    /// Package-owned policy-family resource.
    PackagePolicyFamily,
    /// Package-owned governing-order resource.
    PackageGoverningOrder,
    /// Package-owned capability-policy resource.
    PackageCapabilityPolicy,
    /// Package-owned tool-recipe resource.
    PackageToolRecipe,
    /// Immutable Package identity selected by private configuration.
    Package,
}

/// Canonical semantic identity with its non-interchangeable namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthoringIdentity {
    kind: AuthoringIdentityKind,
    value: String,
    completion_scope: String,
    authored_value: String,
}

impl AuthoringIdentity {
    /// Return the closed namespace that interprets [`Self::value`].
    #[must_use]
    pub const fn kind(&self) -> AuthoringIdentityKind {
        self.kind
    }

    /// Borrow the canonical display form selected by the semantic owner.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Borrow the exact scalar spelling used by same-scope references.
    #[must_use]
    pub fn authored_value(&self) -> &str {
        &self.authored_value
    }
}

/// Semantic role played by one exact authored occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthoringOccurrenceRole {
    /// The occurrence establishes the canonical local source definition.
    Definition,
    /// The occurrence resolves to a separately defined identity.
    Reference,
    /// The occurrence carries a content-bearing interface projection.
    Projection,
}

/// Default work or result limit for one reverse occurrence page.
pub const DEFAULT_AUTHORING_OCCURRENCE_PAGE_LIMIT: usize = 64;
/// Maximum work or result limit for one reverse occurrence page.
pub const MAX_AUTHORING_OCCURRENCE_PAGE_LIMIT: usize = 256;
/// Maximum UTF-8 bytes copied into occurrence locations by one page.
///
/// One returned location contains one admitted source label and field names
/// decoded from one bounded source member or selected from the compiler's fixed
/// path vocabulary. Intake asserts that their aggregate fits this derived
/// ceiling. Array indexes, roles, identities, cursor state, vector storage, and
/// later language-binding copies are not counted.
pub const MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES: usize =
    WORKSPACE_FILE_LIMIT.max_bytes() + MAX_CONFINED_RELATIVE_PATH_BYTES;

/// Closed class of deterministic source correction proved by the compiler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoringCorrectionKind {
    /// Narrow an import visibility ceiling to the selected export's ceiling.
    NarrowImportVisibility,
}

/// Source-free, precondition-fenced edit evidence from rejected semantics.
///
/// The linker emits this value only after resolving the exact upstream export
/// and proving the lattice violation
/// an import ceiling wider than the export visibility. An editor may apply the replacement
/// only when the current scalar bytes still equal [`Self::expected_source`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringCorrectionEvidence {
    kind: AuthoringCorrectionKind,
    source_label: String,
    path: Vec<AuthoringInstancePathSegment>,
    expected_source: String,
    replacement_source: String,
    proof_subject: AuthoringIdentity,
}

impl AuthoringCorrectionEvidence {
    /// Return the compiler-proved correction class.
    #[must_use]
    pub const fn kind(&self) -> AuthoringCorrectionKind {
        self.kind
    }

    /// Borrow the compiler logical source label owning the scalar.
    #[must_use]
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Borrow the exact decoded-key and array-index path.
    #[must_use]
    pub fn path(&self) -> &[AuthoringInstancePathSegment] {
        &self.path
    }

    /// Borrow the exact TOML scalar bytes required before replacement.
    #[must_use]
    pub fn expected_source(&self) -> &str {
        &self.expected_source
    }

    /// Borrow the exact compiler-selected TOML replacement bytes.
    #[must_use]
    pub fn replacement_source(&self) -> &str {
        &self.replacement_source
    }

    /// Borrow the resolved semantic subject proving the correction.
    #[must_use]
    pub const fn proof_subject(&self) -> &AuthoringIdentity {
        &self.proof_subject
    }
}

impl WorkspaceError {
    /// Project an exact safe correction when this failure proves one.
    ///
    /// Most semantic failures intentionally return `None`: absence means the
    /// compiler has not proved one deterministic source transformation.
    #[must_use]
    pub fn authoring_correction(&self) -> Option<AuthoringCorrectionEvidence> {
        let Self::ImportVisibilityWidening {
            source_label,
            importer_module: _,
            import_index,
            upstream_module,
            export_id,
            export_visibility,
            import_ceiling,
        } = self
        else {
            return None;
        };
        Some(AuthoringCorrectionEvidence {
            kind: AuthoringCorrectionKind::NarrowImportVisibility,
            source_label: source_label.clone(),
            path: path([
                field("module"),
                field("imports"),
                at(*import_index),
                field("visibility_ceiling"),
            ]),
            expected_source: format!("\"{import_ceiling}\""),
            replacement_source: format!("\"{export_visibility}\""),
            proof_subject: identity(
                AuthoringIdentityKind::Export,
                format!("{upstream_module}::{export_id}"),
            ),
        })
    }
}

/// One source-free semantic occurrence retained after successful linking.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthoringOccurrence {
    identity: AuthoringIdentity,
    source_label: String,
    path: Vec<AuthoringInstancePathSegment>,
    role: AuthoringOccurrenceRole,
}

impl AuthoringOccurrence {
    /// Borrow the canonical identity resolved at this occurrence.
    #[must_use]
    pub const fn identity(&self) -> &AuthoringIdentity {
        &self.identity
    }

    /// Borrow the compiler logical label of the owning source document.
    #[must_use]
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Borrow the exact decoded-key and array-index path.
    #[must_use]
    pub fn path(&self) -> &[AuthoringInstancePathSegment] {
        &self.path
    }

    /// Return whether this occurrence defines, references, or projects meaning.
    #[must_use]
    pub const fn role(&self) -> AuthoringOccurrenceRole {
        self.role
    }

    fn copied_string_bytes(&self) -> usize {
        location_copied_string_bytes(&self.source_label, &self.path)
    }
}

type AuthoringSourceCoordinate = (String, Vec<AuthoringInstancePathSegment>);

/// Exact boundary from which a later source scan resumes.
///
/// This cursor contains no source text and is meaningful only for the immutable
/// semantic index that issued it. It carries no authority, disclosure, or
/// compilation-identity proof; an outer boundary must bind those dependencies
/// before accepting a continuation from a caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringOccurrencePageCursor {
    coordinate: AuthoringSourceCoordinate,
    boundary: AuthoringOccurrenceResumeBoundary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthoringOccurrenceResumeBoundary {
    After,
    At,
}

impl AuthoringOccurrencePageCursor {
    fn after(coordinate: &AuthoringSourceCoordinate) -> Self {
        Self {
            coordinate: coordinate.clone(),
            boundary: AuthoringOccurrenceResumeBoundary::After,
        }
    }

    fn at(coordinate: &AuthoringSourceCoordinate) -> Self {
        Self {
            coordinate: coordinate.clone(),
            boundary: AuthoringOccurrenceResumeBoundary::At,
        }
    }

    /// Borrow the compiler logical source label at the resume boundary.
    #[must_use]
    pub fn source_label(&self) -> &str {
        &self.coordinate.0
    }

    /// Borrow the exact decoded-key and array-index path at the resume boundary.
    #[must_use]
    pub fn path(&self) -> &[AuthoringInstancePathSegment] {
        &self.coordinate.1
    }
}

/// One work-bounded reverse lookup over an immutable semantic index.
///
/// An empty page can remain incomplete when the inspected source coordinates
/// do not contain the requested identity. A matching row that would cross the
/// copied-string ceiling is left at the continuation boundary and deliberately
/// re-inspected on the next page. Rows are never truncated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringOccurrencePage<'a> {
    occurrences: Vec<&'a AuthoringOccurrence>,
    inspected: usize,
    copied_string_bytes: usize,
    continuation: Option<AuthoringOccurrencePageCursor>,
}

impl<'a> AuthoringOccurrencePage<'a> {
    /// Borrow matching occurrences in deterministic source/path order.
    #[must_use]
    pub fn occurrences(&self) -> &[&'a AuthoringOccurrence] {
        &self.occurrences
    }

    /// Return the number of source coordinates examined for this page.
    #[must_use]
    pub const fn inspected(&self) -> usize {
        self.inspected
    }

    /// Return UTF-8 bytes copied into returned source labels and field names.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> usize {
        self.copied_string_bytes
    }

    /// Borrow the exact continuation boundary selected by the producer.
    #[must_use]
    pub const fn continuation(&self) -> Option<&AuthoringOccurrencePageCursor> {
        self.continuation.as_ref()
    }

    /// Return whether the producer reached the end of the retained index.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.continuation.is_none()
    }
}

/// Source-free definition/reference index produced by canonical admission.
///
/// The index observes already linked workspace declarations and already parsed,
/// qualified Package declarations; it is not a registry or semantic owner.
/// Editor clients use exact instance paths to recover source ranges from their
/// own retained buffers. Source bytes are still discarded after successful
/// admission.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthoringSemanticIndex {
    occurrences: BTreeMap<AuthoringSourceCoordinate, AuthoringOccurrence>,
    /// One deterministic admitted occurrence for every completion-eligible identity.
    candidates: BTreeMap<AuthoringIdentity, AuthoringCandidate>,
    definitions: BTreeMap<AuthoringIdentity, AuthoringOccurrence>,
    references: BTreeMap<AuthoringIdentity, BTreeSet<AuthoringOccurrence>>,
    completion_domains:
        BTreeMap<(String, Vec<AuthoringPathSegment>), BTreeSet<AuthoringCompletionDomain>>,
    /// Owning record coordinates, separate from lexical named occurrences.
    record_anchors: BTreeMap<WorkspaceDerivationSubjectV1, WorkspaceAuthoringLocation>,
}

/// Incompatible custody while composing already admitted compiler phases.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum AuthoringSemanticMergeError {
    /// Preserve the existing source-coordinate overlap failure verbatim.
    #[error("logical source label `{0}` owns overlapping semantic coordinates")]
    OverlappingSourceCoordinate(String),
    /// One complete proof subject cannot acquire a different record anchor.
    #[error("one compiler proof subject owns conflicting record anchors")]
    ConflictingRecordAnchor,
}

/// One completion candidate and, when supported, the proof subject it names.
/// Boxing stores the full subject only once per identity, not in every source row.
#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthoringCandidate {
    occurrence: AuthoringOccurrence,
    proof_subject: Option<Box<WorkspaceDerivationSubjectV1>>,
}

impl AuthoringCandidate {
    /// Typed denotation constructors make one authoring key name one subject.
    /// A mismatch is an internal producer defect, not a source-admission choice.
    fn retain_subject(&mut self, subject: WorkspaceDerivationSubjectV1) {
        if let Some(existing) = &self.proof_subject {
            assert_eq!(
                existing.as_ref(),
                &subject,
                "authoring denotation changed for one identity"
            );
        } else {
            self.proof_subject = Some(Box::new(subject));
        }
    }
}

/// Constructs authoring and proof identities together from typed producer values.
/// Admitted IDs cannot contain `::`, so qualified authoring keys preserve the
/// owner and local identity used by their corresponding proof subjects.
struct DenotedAuthoringIdentity {
    identity: AuthoringIdentity,
    subject: WorkspaceDerivationSubjectV1,
}

impl DenotedAuthoringIdentity {
    fn source(source: &WorkspaceDerivationSourceV1, grammar: &str) -> Self {
        Self {
            identity: source_document_identity(source.source_path.as_str(), grammar),
            subject: WorkspaceDerivationSubjectV1::Source {
                source_id: source.source_id.clone(),
            },
        }
    }

    fn workspace(workspace_id: &WorkspaceId) -> Self {
        Self {
            identity: simple_identity(AuthoringIdentityKind::Workspace, workspace_id.to_string()),
            subject: WorkspaceDerivationSubjectV1::Workspace {
                workspace_id: workspace_id.clone(),
            },
        }
    }

    fn module(module_id: &WorkspaceModuleId) -> Self {
        Self {
            identity: module_identity(module_id.as_ref()),
            subject: WorkspaceDerivationSubjectV1::Module {
                module_id: module_id.clone(),
            },
        }
    }

    fn component(owner: &WorkspaceModuleId, component_id: &WorkspaceComponentId) -> Self {
        Self {
            identity: identity(
                AuthoringIdentityKind::Component,
                format!("{owner}::{component_id}"),
            ),
            subject: WorkspaceDerivationSubjectV1::Component {
                component_id: component_id.clone(),
            },
        }
    }

    fn declaration(declaration: &QualifiedDeclarationRef) -> Self {
        Self {
            identity: declaration_identity(declaration),
            subject: WorkspaceDerivationSubjectV1::Declaration {
                owner_module: declaration.owner_module.clone(),
                declaration: declaration.local_ref.clone(),
            },
        }
    }

    fn resource(owner: &WorkspaceModuleId, resource_id: &WorkspaceResourceId) -> Self {
        Self {
            identity: identity(
                AuthoringIdentityKind::Resource,
                format!("{owner}::{resource_id}"),
            ),
            subject: WorkspaceDerivationSubjectV1::Resource {
                owner_module: owner.clone(),
                resource_id: resource_id.clone(),
            },
        }
    }
}

/// Identity namespace and owner scope eligible at one structural source path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AuthoringCompletionDomain {
    kind: AuthoringIdentityKind,
    completion_scope: String,
}

impl AuthoringSemanticIndex {
    /// Build navigation evidence from the already resolved workspace graph.
    pub(super) fn from_linked(linked: &mut LinkedWorkspace) -> Self {
        let mut index = Self::default();
        index.index_source_documents(linked);
        index.index_topology(linked);
        index.index_components(linked);
        index.index_modules(linked);
        index.index_responsibility(linked);
        index.index_organization(linked);
        index.index_composition(linked);
        index.index_locks(linked);
        index.record_anchors = std::mem::take(&mut linked.record_anchors);
        index
    }

    /// Borrow an owning Import/Extension record coordinate by complete subject.
    /// Absence means no supported retained anchor, not that the subject is absent.
    pub(super) fn subject_record_anchor(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
    ) -> Option<&WorkspaceAuthoringLocation> {
        self.record_anchors.get(subject)
    }

    /// Resolve the semantic occurrence at one exact authored instance path.
    #[must_use]
    pub fn occurrence(
        &self,
        source_label: &str,
        path: &[AuthoringInstancePathSegment],
    ) -> Option<&AuthoringOccurrence> {
        self.occurrences
            .get(&(source_label.to_owned(), path.to_vec()))
    }

    /// Borrow the direct subject registered for this full authoring identity.
    /// The candidate map shares one typed key across its source occurrences.
    pub(super) fn proof_subject(
        &self,
        identity: &AuthoringIdentity,
    ) -> Option<&WorkspaceDerivationSubjectV1> {
        self.candidates.get(identity)?.proof_subject.as_deref()
    }

    /// Scan one bounded page of occurrences for an exact semantic identity.
    ///
    /// Both row limits are independent: zero selects 64 and values above 256
    /// are capped at 256. Every source coordinate is charged before its identity
    /// is compared. Matching rows also share one fixed copied-string ceiling.
    /// A row-limit stop resumes after the last charged coordinate. A byte-limit
    /// stop resumes at the deferred matching coordinate so it is returned once.
    ///
    /// The caller must bind a continuation to the immutable index and admitted
    /// disclosure scope that produced it. Exact identity matching here does not
    /// establish either property.
    #[must_use]
    pub fn occurrences_page<'a>(
        &'a self,
        identity: &AuthoringIdentity,
        after: Option<&AuthoringOccurrencePageCursor>,
        maximum_inspected: usize,
        maximum_returned: usize,
    ) -> AuthoringOccurrencePage<'a> {
        self.matching_occurrences_page(after, maximum_inspected, maximum_returned, |occurrence| {
            occurrence.identity() == identity
        })
    }

    /// Scan direct named occurrences, including captured but unselected subjects.
    /// Each charged row performs a full-identity lookup in the candidate map.
    pub(super) fn subject_occurrences_page<'a>(
        &'a self,
        subject: &WorkspaceDerivationSubjectV1,
        after: Option<&AuthoringOccurrencePageCursor>,
        maximum_inspected: usize,
        maximum_returned: usize,
    ) -> AuthoringOccurrencePage<'a> {
        self.matching_occurrences_page(after, maximum_inspected, maximum_returned, |occurrence| {
            self.proof_subject(occurrence.identity()) == Some(subject)
        })
    }

    fn matching_occurrences_page(
        &self,
        after: Option<&AuthoringOccurrencePageCursor>,
        maximum_inspected: usize,
        maximum_returned: usize,
        matches: impl Fn(&AuthoringOccurrence) -> bool,
    ) -> AuthoringOccurrencePage<'_> {
        let maximum_inspected = normalized_occurrence_page_limit(maximum_inspected);
        let maximum_returned = normalized_occurrence_page_limit(maximum_returned);
        let start = after.map_or(Bound::Unbounded, |cursor| match cursor.boundary {
            AuthoringOccurrenceResumeBoundary::After => Bound::Excluded(&cursor.coordinate),
            AuthoringOccurrenceResumeBoundary::At => Bound::Included(&cursor.coordinate),
        });
        let mut occurrences = Vec::with_capacity(maximum_returned.min(maximum_inspected));
        let mut inspected = 0;
        let mut copied_string_bytes: usize = 0;
        let mut continuation = None;

        for (coordinate, occurrence) in self
            .occurrences
            .range::<AuthoringSourceCoordinate, _>((start, Bound::Unbounded))
        {
            inspected += 1;
            if matches(occurrence) {
                let row_bytes = occurrence.copied_string_bytes();
                assert!(
                    row_bytes <= MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES,
                    "one admitted occurrence exceeds the fixed copied-string ceiling"
                );
                let next_bytes = copied_string_bytes
                    .checked_add(row_bytes)
                    .expect("occurrence copied-string accounting overflowed");
                if next_bytes > MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES {
                    continuation = Some(AuthoringOccurrencePageCursor::at(coordinate));
                    break;
                }
                occurrences.push(occurrence);
                copied_string_bytes = next_bytes;
            }
            if inspected == maximum_inspected || occurrences.len() == maximum_returned {
                continuation = Some(AuthoringOccurrencePageCursor::after(coordinate));
                break;
            }
        }

        AuthoringOccurrencePage {
            occurrences,
            inspected,
            copied_string_bytes,
            continuation,
        }
    }

    /// Borrow the canonical source definition, preferring source components to
    /// equivalent content-bearing export projections.
    #[must_use]
    pub fn definition(&self, identity: &AuthoringIdentity) -> Option<&AuthoringOccurrence> {
        self.definitions.get(identity)
    }

    /// Iterate all resolved references in deterministic source/path order.
    pub fn references(
        &self,
        identity: &AuthoringIdentity,
    ) -> impl Iterator<Item = &AuthoringOccurrence> {
        self.references.get(identity).into_iter().flatten()
    }

    /// Return the exact number of resolved references for one identity.
    #[must_use]
    pub fn reference_count(&self, identity: &AuthoringIdentity) -> usize {
        self.references.get(identity).map_or(0, BTreeSet::len)
    }

    /// Iterate canonical definitions in one exact semantic namespace.
    pub fn definitions(
        &self,
        kind: AuthoringIdentityKind,
    ) -> impl Iterator<Item = &AuthoringOccurrence> {
        self.definitions
            .iter()
            .filter(move |(identity, _)| identity.kind == kind)
            .map(|(_, occurrence)| occurrence)
    }

    /// Iterate definitions eligible at the same typed reference scope.
    pub fn completion_alternatives<'a>(
        &'a self,
        identity: &'a AuthoringIdentity,
    ) -> impl Iterator<Item = &'a AuthoringOccurrence> + 'a {
        self.candidates
            .iter()
            .filter(move |(candidate, _)| {
                candidate.kind == identity.kind
                    && candidate.completion_scope == identity.completion_scope
            })
            .map(|(_, candidate)| &candidate.occurrence)
    }

    /// Iterate definitions eligible at an admitted structural source path.
    ///
    /// Every concrete array index is normalized to
    /// [`AuthoringPathSegment::ArrayItem`]. The result therefore remains useful
    /// for a newly inserted or incomplete reference whose exact identity has not
    /// been admitted, while returning only domains proved by other admitted
    /// occurrences in the same source. No candidate identity or definition is
    /// fabricated.
    pub fn completion_alternatives_at<'a>(
        &'a self,
        source_label: &str,
        path: &[AuthoringInstancePathSegment],
    ) -> impl Iterator<Item = &'a AuthoringOccurrence> + 'a {
        let coordinate = (source_label.to_owned(), structural_authoring_path(path));
        let domains = self.completion_domains.get(&coordinate);
        self.candidates
            .iter()
            .filter(move |(identity, _)| {
                domains.is_some_and(|domains| {
                    domains
                        .iter()
                        .any(|domain| domain.matches_identity(identity))
                })
            })
            .map(|(_, candidate)| &candidate.occurrence)
    }

    /// Merge source-disjoint compiler phases without changing identity owners.
    pub(crate) fn extend(&mut self, other: Self) -> Result<(), AuthoringSemanticMergeError> {
        if let Some((label, _)) = other
            .occurrences
            .keys()
            .find(|coordinate| self.occurrences.contains_key(*coordinate))
        {
            return Err(AuthoringSemanticMergeError::OverlappingSourceCoordinate(
                label.clone(),
            ));
        }
        if other.record_anchors.iter().any(|(subject, anchor)| {
            self.record_anchors
                .get(subject)
                .is_some_and(|existing| existing != anchor)
        }) {
            return Err(AuthoringSemanticMergeError::ConflictingRecordAnchor);
        }
        self.occurrences.extend(other.occurrences);
        for (identity, candidate) in other.candidates {
            match self.candidates.entry(identity) {
                Entry::Vacant(entry) => {
                    entry.insert(candidate);
                }
                Entry::Occupied(mut entry) => {
                    if let Some(subject) = candidate.proof_subject {
                        entry.get_mut().retain_subject(*subject);
                    }
                }
            }
        }
        for (identity, definition) in other.definitions {
            self.definitions.entry(identity).or_insert(definition);
        }
        for (identity, references) in other.references {
            self.references
                .entry(identity)
                .or_default()
                .extend(references);
        }
        for (coordinate, domains) in other.completion_domains {
            self.completion_domains
                .entry(coordinate)
                .or_default()
                .extend(domains);
        }
        for (subject, anchor) in other.record_anchors {
            self.record_anchors.entry(subject).or_insert(anchor);
        }
        Ok(())
    }

    /// Build private configuration identities after typed admission.
    pub(crate) fn from_private(
        capabilities_label: &str,
        capabilities: &pale_core::records::CapabilitiesFile,
        machine_label: &str,
        machine: &pale_core::records::MachineBindingFile,
        lock_label: &str,
        lock: &pale_core::records::DistributionLockFile,
    ) -> Self {
        let mut index = Self::default();
        for (label, grammar) in [
            (capabilities_label, "capabilities.v1"),
            (machine_label, "machine-binding.v1"),
            (lock_label, "distribution-lock.v1"),
        ] {
            index.define(source_document_identity(label, grammar), label, Vec::new());
        }
        for (role, commands) in &capabilities.repository_commands {
            for capability in commands.commands.keys() {
                index.reference(
                    simple_identity(AuthoringIdentityKind::Capability, capability.to_string()),
                    capabilities_label,
                    path([
                        field("repository_commands"),
                        field(role),
                        field("commands"),
                        field(capability.as_str()),
                    ]),
                );
            }
        }
        if let Some(boundary) = &capabilities.boundary {
            for (root_index, root) in boundary.forbidden_token_authority.iter().enumerate() {
                index.reference(
                    simple_identity(AuthoringIdentityKind::AuthorityRoot, root.clone()),
                    capabilities_label,
                    path([
                        field("boundary"),
                        field("forbidden_token_authority"),
                        at(root_index),
                    ]),
                );
            }
        }
        for (field_name, selectors, kind) in [
            (
                "operations",
                capabilities.semantic.operations.as_slice(),
                AuthoringIdentityKind::PackageOperation,
            ),
            (
                "roles",
                capabilities.semantic.roles.as_slice(),
                AuthoringIdentityKind::PackageRole,
            ),
            (
                "doctrine",
                capabilities.semantic.doctrine.as_slice(),
                AuthoringIdentityKind::PackageDoctrine,
            ),
            (
                "run_modes",
                capabilities.semantic.run_modes.as_slice(),
                AuthoringIdentityKind::PackageRunMode,
            ),
            (
                "stacks",
                capabilities.semantic.stacks.as_slice(),
                AuthoringIdentityKind::PackageStack,
            ),
        ] {
            index.index_package_selectors(capabilities_label, field_name, selectors, kind);
        }
        for (axis, mode) in &capabilities.semantic.mode_defaults {
            index.reference(
                package_resource_identity(mode, AuthoringIdentityKind::PackageRunMode),
                capabilities_label,
                path([
                    field("semantic"),
                    field("mode_defaults"),
                    field(&axis.to_string()),
                    field("resource_id"),
                ]),
            );
        }
        for (capability, binding) in &capabilities.capability_bindings {
            index.reference(
                simple_identity(AuthoringIdentityKind::Capability, capability.to_string()),
                capabilities_label,
                path([field("capability_bindings"), field(capability.as_str())]),
            );
            if let Some(provider_tools) = binding.provider_tools() {
                for (row_index, row) in provider_tools.iter().enumerate() {
                    index.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionProviderBinding,
                            row.provider_binding.provider_binding_id.clone(),
                        ),
                        capabilities_label,
                        path([
                            field("capability_bindings"),
                            field(capability.as_str()),
                            field("implementation"),
                            field("provider_tools"),
                            at(row_index),
                            field("provider_binding"),
                            field("provider_binding_id"),
                        ]),
                    );
                }
            }
        }
        if let Some(narrowing) = &capabilities.capability_narrowing {
            for (field_name, capabilities) in [
                (
                    "permitted_capabilities",
                    narrowing.permitted_capabilities.as_slice(),
                ),
                (
                    "forbidden_capabilities",
                    narrowing.forbidden_capabilities.as_slice(),
                ),
            ] {
                for (capability_index, capability) in capabilities.iter().enumerate() {
                    index.reference(
                        simple_identity(AuthoringIdentityKind::Capability, capability.to_string()),
                        capabilities_label,
                        path([
                            field("capability_narrowing"),
                            field(field_name),
                            at(capability_index),
                        ]),
                    );
                }
            }
        }
        for (binding_index, binding) in capabilities.operation_bindings.iter().enumerate() {
            index.reference(
                package_resource_identity(
                    &binding.operation,
                    AuthoringIdentityKind::PackageOperation,
                ),
                capabilities_label,
                path([
                    field("operation_bindings"),
                    at(binding_index),
                    field("operation"),
                    field("resource_id"),
                ]),
            );
        }
        if let Some(lifecycle) = &capabilities.lifecycle {
            for (behavior_index, behavior) in lifecycle.behaviours.iter().enumerate() {
                let base = path([field("lifecycle"), field("behaviour"), at(behavior_index)]);
                index.define(
                    simple_identity(
                        AuthoringIdentityKind::LifecycleBehavior,
                        behavior.id.to_string(),
                    ),
                    capabilities_label,
                    child(&base, "id"),
                );
                if let Some(mode) = &behavior.activate_mode {
                    index.reference(
                        package_resource_identity(mode, AuthoringIdentityKind::PackageRunMode),
                        capabilities_label,
                        nested(&base, [field("activate_mode"), field("resource_id")]),
                    );
                }
                for (mode_index, mode) in behavior.modes.iter().enumerate() {
                    index.reference(
                        package_resource_identity(mode, AuthoringIdentityKind::PackageRunMode),
                        capabilities_label,
                        nested(
                            &base,
                            [field("modes"), at(mode_index), field("resource_id")],
                        ),
                    );
                }
                for (requirement_index, capability) in behavior.tool_reqs.iter().enumerate() {
                    index.reference(
                        simple_identity(AuthoringIdentityKind::Capability, capability.to_string()),
                        capabilities_label,
                        nested(&base, [field("tool_reqs"), at(requirement_index)]),
                    );
                }
            }
        }
        for (policy_index, policy) in capabilities.run_policies.iter().enumerate() {
            index.define(
                simple_identity(AuthoringIdentityKind::RunPolicy, policy.id.to_string()),
                capabilities_label,
                path([field("run_policies"), at(policy_index), field("id")]),
            );
        }
        index.define(
            simple_identity(AuthoringIdentityKind::Machine, machine.machine.id.clone()),
            machine_label,
            path([field("machine"), field("id")]),
        );
        for (checkout_index, repository) in machine.repository_checkouts.iter().enumerate() {
            index.reference(
                simple_identity(
                    AuthoringIdentityKind::Repository,
                    repository.repository_id.to_string(),
                ),
                machine_label,
                path([
                    field("repository_checkouts"),
                    at(checkout_index),
                    field("repository_id"),
                ]),
            );
        }
        for (projection_index, repository) in machine.provider_repositories.iter().enumerate() {
            index.reference(
                simple_identity(
                    AuthoringIdentityKind::Repository,
                    repository.repository_id.to_string(),
                ),
                machine_label,
                path([
                    field("provider_repositories"),
                    at(projection_index),
                    field("repository_id"),
                ]),
            );
            index.reference(
                simple_identity(
                    AuthoringIdentityKind::CompositionProviderBinding,
                    repository.provider_binding_id.clone(),
                ),
                machine_label,
                path([
                    field("provider_repositories"),
                    at(projection_index),
                    field("provider_binding_id"),
                ]),
            );
        }
        for root in machine.authority_roots.keys() {
            index.define(
                simple_identity(AuthoringIdentityKind::AuthorityRoot, root.clone()),
                machine_label,
                path([field("authority_roots"), field(root)]),
            );
        }
        for (binding_index, binding) in machine.native_harnesses.iter().enumerate() {
            index.define(
                simple_identity(
                    AuthoringIdentityKind::ProviderBinding,
                    binding.provider_binding.provider_binding_id.clone(),
                ),
                machine_label,
                path([
                    field("native_harnesses"),
                    at(binding_index),
                    field("provider_binding"),
                    field("provider_binding_id"),
                ]),
            );
        }
        if let Some(preferences) = &machine.preferences {
            for (axis, mode) in &preferences.mode_defaults {
                index.reference(
                    package_resource_identity(mode, AuthoringIdentityKind::PackageRunMode),
                    machine_label,
                    path([
                        field("preferences"),
                        field("mode_defaults"),
                        field(&axis.to_string()),
                        field("resource_id"),
                    ]),
                );
            }
        }
        for (package_index, package) in lock.packages.iter().enumerate() {
            index.reference(
                simple_identity(AuthoringIdentityKind::Package, package.id.to_string()),
                lock_label,
                path([field("package"), at(package_index), field("id")]),
            );
        }
        index
    }

    /// Build Package-owner definitions from the admitted manifest closure.
    ///
    /// Resource definitions are deliberately added later by Package admission,
    /// where the already parsed declaration owns its exact substantive source
    /// coordinate. A manifest inventory row declares custody; it is not the
    /// resource's authoring definition.
    pub(crate) fn from_package_manifests(
        closure: &crate::packages::ResourceSet<'_>,
    ) -> Result<Self, String> {
        let mut index = Self::default();
        for (package_id, package) in closure.packages() {
            let label = package.manifest_label()?;
            index.define(
                simple_identity(AuthoringIdentityKind::Package, package_id.to_string()),
                &label,
                path([field("package"), field("id")]),
            );
            for (resource_index, resource) in
                package.manifest().package.resources.iter().enumerate()
            {
                let resource_ref = pale_core::records::PackageResourceRefV1 {
                    package_id: package_id.clone(),
                    resource_id: resource.id.clone(),
                };
                index.reference(
                    package_resource_identity(&resource_ref, package_identity_kind(resource.kind)),
                    &label,
                    path([
                        field("package"),
                        field("resources"),
                        at(resource_index),
                        field("id"),
                    ]),
                );
            }
        }
        Ok(index)
    }

    /// Retain one successfully admitted Package resource at its authored source.
    pub(crate) fn define_package_resource(
        &mut self,
        resource: &pale_core::records::PackageResourceRefV1,
        kind: pale_core::records::PackageResourceKindV1,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
    ) {
        self.define(
            package_resource_identity(resource, package_identity_kind(kind)),
            source_label,
            path,
        );
    }

    /// Retain one successfully qualified Package-resource relationship.
    pub(crate) fn reference_package_resource(
        &mut self,
        resource: &pale_core::records::PackageResourceRefV1,
        kind: pale_core::records::PackageResourceKindV1,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
    ) {
        self.reference(
            package_resource_identity(resource, package_identity_kind(kind)),
            source_label,
            path,
        );
    }

    /// Retain one Package-authored use of a portable capability identity.
    pub(crate) fn reference_package_capability(
        &mut self,
        capability: &pale_core::records::CapabilityIdV1,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
    ) {
        self.reference(
            simple_identity(AuthoringIdentityKind::Capability, capability.to_string()),
            source_label,
            path,
        );
    }

    fn index_package_selectors(
        &mut self,
        label: &str,
        field_name: &str,
        selectors: &[pale_core::records::PackageResourceSelectorV1],
        kind: AuthoringIdentityKind,
    ) {
        for (selector_index, selector) in selectors.iter().enumerate() {
            let base = path([field("semantic"), field(field_name), at(selector_index)]);
            match selector {
                pale_core::records::PackageResourceSelectorV1::Exact { resource } => {
                    self.reference(
                        package_resource_identity(resource, kind),
                        label,
                        nested(&base, [field("resource"), field("resource_id")]),
                    );
                }
                pale_core::records::PackageResourceSelectorV1::AllFromPackage { package_id } => {
                    self.reference(
                        simple_identity(AuthoringIdentityKind::Package, package_id.to_string()),
                        label,
                        child(&base, "package_id"),
                    );
                }
            }
        }
    }

    fn insert(
        &mut self,
        identity: AuthoringIdentity,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
        role: AuthoringOccurrenceRole,
    ) {
        self.insert_with_subject(identity, source_label, path, role, None);
    }

    fn insert_denoted(
        &mut self,
        denotation: DenotedAuthoringIdentity,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
        role: AuthoringOccurrenceRole,
    ) {
        self.insert_with_subject(
            denotation.identity,
            source_label,
            path,
            role,
            Some(denotation.subject),
        );
    }

    fn insert_with_subject(
        &mut self,
        identity: AuthoringIdentity,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
        role: AuthoringOccurrenceRole,
        subject: Option<WorkspaceDerivationSubjectV1>,
    ) {
        assert!(
            source_label.len() <= MAX_CONFINED_RELATIVE_PATH_BYTES,
            "admitted authoring source label exceeds its owner bound"
        );
        assert!(
            location_copied_string_bytes(source_label, &path)
                <= MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES,
            "authoring field path is not bounded by its admitted source member"
        );
        let occurrence = AuthoringOccurrence {
            identity: identity.clone(),
            source_label: source_label.to_owned(),
            path: path.clone(),
            role,
        };
        self.completion_domains
            .entry((source_label.to_owned(), structural_authoring_path(&path)))
            .or_default()
            .insert(AuthoringCompletionDomain::from_identity(&identity));
        self.occurrences
            .insert((source_label.to_owned(), path), occurrence.clone());
        let candidate =
            self.candidates
                .entry(identity.clone())
                .or_insert_with(|| AuthoringCandidate {
                    occurrence: occurrence.clone(),
                    proof_subject: None,
                });
        if let Some(subject) = subject {
            candidate.retain_subject(subject);
        }
        match role {
            AuthoringOccurrenceRole::Definition => {
                self.definitions.entry(identity).or_insert(occurrence);
            }
            AuthoringOccurrenceRole::Reference | AuthoringOccurrenceRole::Projection => {
                self.references
                    .entry(identity.clone())
                    .or_default()
                    .insert(occurrence.clone());
                if role == AuthoringOccurrenceRole::Projection {
                    self.definitions.entry(identity).or_insert(occurrence);
                }
            }
        }
    }

    fn index_topology(&mut self, linked: &LinkedWorkspace) {
        self.index_project_context(linked);
        let label = linked.source.manifest_source.source_path.as_str();
        let workspace = &linked.source.manifest.workspace;
        let topology = &workspace.desired_topology;
        self.define_denoted(
            DenotedAuthoringIdentity::workspace(&workspace.workspace_id),
            label,
            path([field("workspace"), field("workspace_id")]),
        );
        let topology_base = path([field("workspace"), field("desired_topology")]);
        for (index, project) in topology.projects.iter().enumerate() {
            let base = nested(&topology_base, [field("projects"), at(index)]);
            self.define(
                simple_identity(AuthoringIdentityKind::Project, project.id.to_string()),
                label,
                child(&base, "id"),
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::RunPolicy,
                    project.run_policy.to_string(),
                ),
                label,
                child(&base, "run_policy"),
            );
            self.reference(
                source_document_identity(project.capabilities_file.as_str(), "capabilities.v1"),
                label,
                child(&base, "capabilities_file"),
            );
            for (seed_index, package) in project.package_seeds.iter().enumerate() {
                self.reference(
                    simple_identity(AuthoringIdentityKind::Package, package.to_string()),
                    label,
                    nested(&base, [field("package_seeds"), at(seed_index)]),
                );
            }
            for (root_index, root) in project.authority_root_ids.iter().enumerate() {
                self.reference(
                    simple_identity(AuthoringIdentityKind::AuthorityRoot, root.clone()),
                    label,
                    nested(&base, [field("authority_root_ids"), at(root_index)]),
                );
            }
        }
        for (index, execution_workspace) in topology.execution_workspaces.iter().enumerate() {
            let base = nested(&topology_base, [field("execution_workspaces"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::ExecutionWorkspace,
                    execution_workspace.id.to_string(),
                ),
                label,
                child(&base, "id"),
            );
            for (path_index, path_id) in execution_workspace.execution_path_ids.iter().enumerate() {
                self.reference(
                    simple_identity(AuthoringIdentityKind::ExecutionPath, path_id.to_string()),
                    label,
                    nested(&base, [field("execution_path_ids"), at(path_index)]),
                );
            }
            if let Some(session) = &execution_workspace.session {
                let session_base = child(&base, "session");
                for (field_name, repositories) in [
                    ("activation_roots", session.activation_roots.as_slice()),
                    ("additional_roots", session.additional_roots.as_slice()),
                ] {
                    for (repository_index, repository) in repositories.iter().enumerate() {
                        self.reference(
                            simple_identity(
                                AuthoringIdentityKind::Repository,
                                repository.to_string(),
                            ),
                            label,
                            nested(&session_base, [field(field_name), at(repository_index)]),
                        );
                    }
                }
                for (access_index, access) in session.authority_access.iter().enumerate() {
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::AuthorityRoot,
                            access.root_id.clone(),
                        ),
                        label,
                        nested(
                            &session_base,
                            [
                                field("authority_access"),
                                at(access_index),
                                field("root_id"),
                            ],
                        ),
                    );
                }
                for (path_index, path_id) in session.activation_path_ids.iter().enumerate() {
                    self.reference(
                        simple_identity(AuthoringIdentityKind::ExecutionPath, path_id.to_string()),
                        label,
                        nested(
                            &session_base,
                            [field("activation_path_ids"), at(path_index)],
                        ),
                    );
                }
                for (entry_index, entry) in session.reading_order.iter().enumerate() {
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::AuthorityRoot,
                            entry.root_id.clone(),
                        ),
                        label,
                        nested(
                            &session_base,
                            [field("reading_order"), at(entry_index), field("root_id")],
                        ),
                    );
                }
            }
        }
        for (index, repository) in topology.repositories.iter().enumerate() {
            self.define(
                simple_identity(AuthoringIdentityKind::Repository, repository.id.to_string()),
                label,
                nested(
                    &topology_base,
                    [field("repositories"), at(index), field("id")],
                ),
            );
        }
        for (index, execution_path) in topology.execution_paths.iter().enumerate() {
            let base = nested(&topology_base, [field("execution_paths"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::ExecutionPath,
                    execution_path.id.to_string(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::ExperiencePath,
                    execution_path.experience_path_id.clone(),
                ),
                label,
                child(&base, "experience_path_id"),
            );
        }
        for (index, membership) in topology.project_repositories.iter().enumerate() {
            let base = nested(&topology_base, [field("project_repositories"), at(index)]);
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::Project,
                    membership.project_id.to_string(),
                ),
                label,
                child(&base, "project_id"),
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::Repository,
                    membership.repository_id.to_string(),
                ),
                label,
                child(&base, "repository_id"),
            );
            self.reference(
                package_resource_identity(&membership.role, AuthoringIdentityKind::PackageRole),
                label,
                nested(&base, [field("role"), field("resource_id")]),
            );
            for (stack_index, stack) in membership.stacks.iter().enumerate() {
                self.reference(
                    package_resource_identity(stack, AuthoringIdentityKind::PackageStack),
                    label,
                    nested(
                        &base,
                        [field("stacks"), at(stack_index), field("resource_id")],
                    ),
                );
            }
            for (family_index, family) in membership.families.iter().enumerate() {
                self.reference(
                    package_resource_identity(family, AuthoringIdentityKind::PackagePolicyFamily),
                    label,
                    nested(
                        &base,
                        [field("families"), at(family_index), field("resource_id")],
                    ),
                );
            }
        }
        for (index, membership) in topology.project_execution_workspaces.iter().enumerate() {
            let base = nested(
                &topology_base,
                [field("project_execution_workspaces"), at(index)],
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::Project,
                    membership.project_id.to_string(),
                ),
                label,
                child(&base, "project_id"),
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::ExecutionWorkspace,
                    membership.execution_workspace_id.to_string(),
                ),
                label,
                child(&base, "execution_workspace_id"),
            );
        }
        for (index, membership) in topology.execution_workspace_repositories.iter().enumerate() {
            let base = nested(
                &topology_base,
                [field("execution_workspace_repositories"), at(index)],
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::ExecutionWorkspace,
                    membership.execution_workspace_id.to_string(),
                ),
                label,
                child(&base, "execution_workspace_id"),
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::Repository,
                    membership.repository_id.to_string(),
                ),
                label,
                child(&base, "repository_id"),
            );
        }
    }

    fn index_source_documents(&mut self, linked: &LinkedWorkspace) {
        let source = &linked.source;
        let root_label = source.manifest_source.source_path.as_str();
        self.define_denoted(
            DenotedAuthoringIdentity::source(&source.manifest_source, "pale-workspace.v1"),
            root_label,
            Vec::new(),
        );
        for retained in source.modules.values() {
            let label = retained.source.source_path.as_str();
            self.define_denoted(
                DenotedAuthoringIdentity::source(&retained.source, "workspace-module.v1"),
                label,
                Vec::new(),
            );
        }
        for retained in source.components.values() {
            let label = retained.source.source_path.as_str();
            self.define_denoted(
                DenotedAuthoringIdentity::source(
                    &retained.source,
                    "workspace-language-primitives.v1",
                ),
                label,
                Vec::new(),
            );
        }
        for retained in source.responsibility_derivations.values() {
            let label = retained.source.source_path.as_str();
            self.define_denoted(
                DenotedAuthoringIdentity::source(&retained.source, "responsibility-derivation.v1"),
                label,
                Vec::new(),
            );
        }
        if let Some(retained) = &source.organizational_translation {
            let label = retained.source.source_path.as_str();
            self.define_denoted(
                DenotedAuthoringIdentity::source(&retained.source, "organizational-translation.v1"),
                label,
                Vec::new(),
            );
        }
        if let Some(retained) = &source.experience_composition {
            let label = retained.source.source_path.as_str();
            self.define_denoted(
                DenotedAuthoringIdentity::source(
                    &retained.source,
                    "workspace-composition-source.v1",
                ),
                label,
                Vec::new(),
            );
        }
        for (retained, grammar) in [
            (
                &source.resolution_lock.source,
                "workspace-resolution-lock.v1",
            ),
            (&source.universe_lock.source, "workspace-universe-lock.v1"),
        ] {
            self.define_denoted(
                DenotedAuthoringIdentity::source(retained, grammar),
                retained.source_path.as_str(),
                Vec::new(),
            );
        }

        let workspace = &source.manifest.workspace;
        for (field_name, labels, grammar) in [
            (
                "module_manifests",
                workspace.module_manifests.as_slice(),
                "workspace-module.v1",
            ),
            (
                "source_components",
                workspace.source_components.as_slice(),
                "workspace-language-primitives.v1",
            ),
            (
                "responsibility_derivations",
                workspace.responsibility_derivations.as_slice(),
                "responsibility-derivation.v1",
            ),
            (
                "organizational_translations",
                workspace.organizational_translations.as_slice(),
                "organizational-translation.v1",
            ),
            (
                "experience_compositions",
                workspace.experience_compositions.as_slice(),
                "workspace-composition-source.v1",
            ),
        ] {
            for (source_index, source_label) in labels.iter().enumerate() {
                self.reference(
                    source_document_identity(source_label.as_str(), grammar),
                    root_label,
                    path([field("workspace"), field(field_name), at(source_index)]),
                );
            }
        }
        for (field_name, source_label, grammar) in [
            (
                "distribution_lock",
                workspace.distribution_lock.as_str(),
                "distribution-lock.v1",
            ),
            (
                "resolution_lock",
                workspace.resolution_lock.as_str(),
                "workspace-resolution-lock.v1",
            ),
            (
                "universe_lock",
                workspace.universe_lock.as_str(),
                "workspace-universe-lock.v1",
            ),
        ] {
            self.reference(
                source_document_identity(source_label, grammar),
                root_label,
                path([field("workspace"), field(field_name)]),
            );
        }
    }

    fn index_responsibility(&mut self, linked: &LinkedWorkspace) {
        use pale_core::records::AttributableAnswerV1;

        for retained in linked.source.responsibility_derivations.values() {
            let label = retained.source.source_path.as_str();
            let derivation = &retained.value;
            self.define(
                simple_identity(
                    AuthoringIdentityKind::ResponsibilityDerivation,
                    derivation.derivation_id.to_string(),
                ),
                label,
                path([field("derivation_id")]),
            );
            for (projection_index, projection) in derivation.native_projections.iter().enumerate() {
                self.define(
                    simple_identity(
                        AuthoringIdentityKind::ResponsibilityProjection,
                        projection.projection_id.to_string(),
                    ),
                    label,
                    path([
                        field("native_projections"),
                        at(projection_index),
                        field("projection_id"),
                    ]),
                );
            }
            for (link_index, link) in derivation.genealogy_links.iter().enumerate() {
                let base = path([field("genealogy_links"), at(link_index)]);
                self.define(
                    simple_identity(
                        AuthoringIdentityKind::ResponsibilityGenealogyLink,
                        link.link_id.to_string(),
                    ),
                    label,
                    child(&base, "link_id"),
                );
                for (responsibility_index, responsibility) in
                    link.responsibilities.iter().enumerate()
                {
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::ResponsibilityDerivation,
                            responsibility.to_string(),
                        ),
                        label,
                        nested(&base, [field("responsibilities"), at(responsibility_index)]),
                    );
                }
                for (projection_index, projection) in link.projections.iter().enumerate() {
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::ResponsibilityProjection,
                            projection.to_string(),
                        ),
                        label,
                        nested(&base, [field("projections"), at(projection_index)]),
                    );
                }
            }
            if let AttributableAnswerV1::Supplied { value, .. } = &derivation.publisher_module {
                let base = path([field("publisher_module"), field("value")]);
                let owner = value.module_id.to_string();
                self.reference_denoted(
                    DenotedAuthoringIdentity::module(&value.module_id),
                    label,
                    child(&base, "module_id"),
                );
                self.reference(
                    identity(
                        AuthoringIdentityKind::ModuleRelease,
                        format!("{owner}::{}", value.release_id),
                    ),
                    label,
                    child(&base, "release_id"),
                );
                self.reference(
                    identity(
                        AuthoringIdentityKind::Export,
                        format!("{owner}::{}", value.export_id),
                    ),
                    label,
                    child(&base, "export_id"),
                );
            }
            if let AttributableAnswerV1::Supplied { value, .. } = &derivation.adopter_module {
                let base = path([field("adopter_module"), field("value")]);
                let owner = value.module_id.to_string();
                let imported = value.imported_module_id.to_string();
                self.reference_denoted(
                    DenotedAuthoringIdentity::module(&value.module_id),
                    label,
                    child(&base, "module_id"),
                );
                self.reference_denoted(
                    DenotedAuthoringIdentity::module(&value.imported_module_id),
                    label,
                    child(&base, "imported_module_id"),
                );
                self.reference(
                    identity(
                        AuthoringIdentityKind::ModuleRelease,
                        format!("{imported}::{}", value.imported_release_id),
                    ),
                    label,
                    child(&base, "imported_release_id"),
                );
                self.reference(
                    identity(
                        AuthoringIdentityKind::Export,
                        format!("{imported}::{}", value.imported_export_id),
                    ),
                    label,
                    child(&base, "imported_export_id"),
                );
                self.reference(
                    identity(
                        AuthoringIdentityKind::Namespace,
                        format!("{owner}::{}", value.local_namespace),
                    ),
                    label,
                    child(&base, "local_namespace"),
                );
            }
        }
    }

    fn index_organization(&mut self, linked: &LinkedWorkspace) {
        let Some(retained) = &linked.source.organizational_translation else {
            return;
        };
        let label = retained.source.source_path.as_str();
        let source = &retained.value;

        for (index, scope) in source.scopes.iter().enumerate() {
            self.define(
                scope_identity(&scope.scope),
                label,
                path([
                    field("scopes"),
                    at(index),
                    field("scope"),
                    field("scope_id"),
                ]),
            );
        }
        for (index, organization) in source.organizations.iter().enumerate() {
            let base = path([field("organizations"), at(index)]);
            self.define(
                organization_identity(&organization.organization),
                label,
                nested(&base, [field("organization"), field("organization_id")]),
            );
            self.reference(
                scope_identity(&organization.scope),
                label,
                nested(&base, [field("scope"), field("scope_id")]),
            );
        }
        for (index, group) in source.groups.iter().enumerate() {
            let base = path([field("groups"), at(index)]);
            self.define(
                group_identity(&group.group),
                label,
                nested(&base, [field("group"), field("group_id")]),
            );
            self.reference(
                organization_identity(&group.organization),
                label,
                nested(&base, [field("organization"), field("organization_id")]),
            );
        }
        for (index, tree) in source.discipline_trees.iter().enumerate() {
            let base = path([field("discipline_trees"), at(index)]);
            self.define(
                discipline_tree_identity(&tree.tree),
                label,
                nested(&base, [field("tree"), field("tree_id")]),
            );
            self.reference(
                organization_identity(&tree.organization),
                label,
                nested(&base, [field("organization"), field("organization_id")]),
            );
        }
        for (index, role) in source.roles.iter().enumerate() {
            let base = path([field("roles"), at(index)]);
            self.define(
                organizational_role_identity(&role.role),
                label,
                nested(&base, [field("role"), field("role_id")]),
            );
            self.reference(
                organization_identity(&role.organization),
                label,
                nested(&base, [field("organization"), field("organization_id")]),
            );
            if let Some(group) = &role.group {
                self.reference(
                    group_identity(group),
                    label,
                    nested(&base, [field("group"), field("group_id")]),
                );
            }
            if let Some(tree) = &role.discipline_tree {
                self.reference(
                    discipline_tree_identity(tree),
                    label,
                    nested(&base, [field("discipline_tree"), field("tree_id")]),
                );
            }
        }
        for (index, binding) in source.occupant_bindings.iter().enumerate() {
            let base = path([field("occupant_bindings"), at(index)]);
            self.reference(
                organizational_role_identity(&binding.role),
                label,
                nested(&base, [field("role"), field("role_id")]),
            );
            self.define(
                simple_identity(
                    AuthoringIdentityKind::OrganizationalOccupant,
                    binding.occupant.to_string(),
                ),
                label,
                child(&base, "occupant"),
            );
        }
        for (index, binding) in source.provider_bindings.iter().enumerate() {
            let base = path([field("provider_bindings"), at(index)]);
            self.reference(
                organizational_role_identity(&binding.role),
                label,
                nested(&base, [field("role"), field("role_id")]),
            );
            self.define(
                simple_identity(
                    AuthoringIdentityKind::OrganizationalProvider,
                    binding.provider.to_string(),
                ),
                label,
                child(&base, "provider"),
            );
        }
        for (index, binding) in source.communication_actor_bindings.iter().enumerate() {
            let base = path([field("communication_actor_bindings"), at(index)]);
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::OrganizationalProvider,
                    binding.provider.to_string(),
                ),
                label,
                child(&base, "provider"),
            );
            self.define(
                scoped_identity(
                    AuthoringIdentityKind::CommunicationActor,
                    format!("{}::{}", binding.provider, binding.handle),
                    binding.provider.to_string(),
                    binding.handle.to_string(),
                ),
                label,
                child(&base, "handle"),
            );
            for (resolution_index, resolution) in binding.local_resolutions.iter().enumerate() {
                let resolution_base =
                    nested(&base, [field("local_resolutions"), at(resolution_index)]);
                self.index_organization_node(
                    &resolution.local_node,
                    label,
                    &child(&resolution_base, "local_node"),
                );
                for (node_index, node) in resolution.authorized_closure.iter().enumerate() {
                    self.index_organization_node(
                        node,
                        label,
                        &nested(
                            &resolution_base,
                            [field("authorized_closure"), at(node_index)],
                        ),
                    );
                }
            }
        }
        for (index, leadership) in source.leadership.iter().enumerate() {
            let base = path([field("leadership"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::LeadershipTopic,
                    leadership.topic.to_string(),
                ),
                label,
                child(&base, "topic"),
            );
            for (participant_index, participant) in leadership.participants.iter().enumerate() {
                self.reference(
                    organizational_role_identity(participant),
                    label,
                    nested(
                        &base,
                        [
                            field("participants"),
                            at(participant_index),
                            field("role_id"),
                        ],
                    ),
                );
            }
            for (scope_index, scope) in leadership.affected_scopes.iter().enumerate() {
                self.reference(
                    scope_identity(scope),
                    label,
                    nested(
                        &base,
                        [field("affected_scopes"), at(scope_index), field("scope_id")],
                    ),
                );
            }
        }
        for (index, relation) in source.relations.iter().enumerate() {
            let base = path([field("relations"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::OrganizationalRelation,
                    relation.relation_id.to_string(),
                ),
                label,
                child(&base, "relation_id"),
            );
            self.index_organization_node(&relation.from, label, &child(&base, "from"));
            self.index_organization_node(&relation.to, label, &child(&base, "to"));
        }
        for (index, object) in source.objects.iter().enumerate() {
            let base = path([field("objects"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::OrganizationalObject,
                    object.object_id.to_string(),
                ),
                label,
                child(&base, "object_id"),
            );
            self.reference(
                scope_identity(&object.scope),
                label,
                nested(&base, [field("scope"), field("scope_id")]),
            );
            self.reference(
                organization_identity(&object.organization),
                label,
                nested(&base, [field("organization"), field("organization_id")]),
            );
        }
        for (index, profile) in source.group_lenses.iter().enumerate() {
            let base = path([field("group_lenses"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::GroupLensProfile,
                    profile.profile_id.to_string(),
                ),
                label,
                child(&base, "profile_id"),
            );
            self.reference(
                organizational_role_identity(&profile.holder),
                label,
                nested(&base, [field("holder"), field("role_id")]),
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::OrganizationalRelation,
                    profile.alignment_relation.to_string(),
                ),
                label,
                child(&base, "alignment_relation"),
            );
            for (field_name, role) in [
                ("review_owner", profile.review_owner.as_ref()),
                ("escalation_resolver", profile.escalation_resolver.as_ref()),
            ] {
                if let Some(role) = role {
                    self.reference(
                        organizational_role_identity(role),
                        label,
                        nested(&base, [field(field_name), field("role_id")]),
                    );
                }
            }
        }
        for (index, service) in source.services.iter().enumerate() {
            let base = path([field("services"), at(index)]);
            self.define(
                directional_service_identity(&service.contract_ref()),
                label,
                child(&base, "contract_id"),
            );
            self.reference(
                scope_identity(&service.provider_scope),
                label,
                nested(&base, [field("provider_scope"), field("scope_id")]),
            );
            self.reference(
                scope_identity(&service.consumer_scope),
                label,
                nested(&base, [field("consumer_scope"), field("scope_id")]),
            );
        }
        for (index, projection) in source.native_projections.iter().enumerate() {
            let base = path([field("native_projections"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::OrganizationalProjection,
                    projection.projection_id.to_string(),
                ),
                label,
                child(&base, "projection_id"),
            );
            self.index_organization_node(&projection.subject, label, &child(&base, "subject"));
        }
        if let Some(inspection) = &source.inspection {
            self.define(
                simple_identity(
                    AuthoringIdentityKind::OrganizationalInspection,
                    inspection.inspection_id.to_string(),
                ),
                label,
                path([field("inspection"), field("inspection_id")]),
            );
        }
    }

    fn index_organization_node(
        &mut self,
        node: &pale_core::records::OrganizationalNodeRefV1,
        label: &str,
        base: &[AuthoringInstancePathSegment],
    ) {
        use pale_core::records::OrganizationalNodeRefV1;
        let (identity, tail) = match node {
            OrganizationalNodeRefV1::Scope { scope } => (
                scope_identity(scope),
                path([field("scope"), field("scope_id")]),
            ),
            OrganizationalNodeRefV1::Organization { organization } => (
                organization_identity(organization),
                path([field("organization"), field("organization_id")]),
            ),
            OrganizationalNodeRefV1::Group { group } => (
                group_identity(group),
                path([field("group"), field("group_id")]),
            ),
            OrganizationalNodeRefV1::DisciplineTree { tree } => (
                discipline_tree_identity(tree),
                path([field("tree"), field("tree_id")]),
            ),
            OrganizationalNodeRefV1::Role { role } => (
                organizational_role_identity(role),
                path([field("role"), field("role_id")]),
            ),
            OrganizationalNodeRefV1::Occupant { occupant } => (
                simple_identity(
                    AuthoringIdentityKind::OrganizationalOccupant,
                    occupant.to_string(),
                ),
                path([field("occupant")]),
            ),
            OrganizationalNodeRefV1::Provider { provider } => (
                simple_identity(
                    AuthoringIdentityKind::OrganizationalProvider,
                    provider.to_string(),
                ),
                path([field("provider")]),
            ),
            OrganizationalNodeRefV1::Object { object } => (
                simple_identity(
                    AuthoringIdentityKind::OrganizationalObject,
                    object.to_string(),
                ),
                path([field("object")]),
            ),
            OrganizationalNodeRefV1::NativeResource { .. } => return,
        };
        self.reference(identity, label, base.iter().cloned().chain(tail).collect());
    }

    fn index_composition(&mut self, linked: &LinkedWorkspace) {
        let Some(retained) = &linked.source.experience_composition else {
            return;
        };
        let label = retained.source.source_path.as_str();
        let declarations = &retained.value.declarations;
        let root = path([field("declarations")]);

        for (index, contract) in declarations.responsibility_contracts.iter().enumerate() {
            let base = nested(&root, [field("responsibility_contracts"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionResponsibilityContract,
                    contract.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference_composition_participant(
                &contract.participant,
                label,
                &child(&base, "participant"),
            );
        }
        for (manifest_index, manifest) in declarations
            .participant_provider_manifests
            .iter()
            .enumerate()
        {
            let base = nested(
                &root,
                [field("participant_provider_manifests"), at(manifest_index)],
            );
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionParticipantManifest,
                    manifest.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            for (participant_index, participant) in manifest.participants.iter().enumerate() {
                self.define_composition_participant(
                    participant,
                    label,
                    &nested(&base, [field("participants"), at(participant_index)]),
                );
            }
            for (provider_index, provider) in manifest.providers.iter().enumerate() {
                let provider_base = nested(&base, [field("providers"), at(provider_index)]);
                self.define(
                    simple_identity(
                        AuthoringIdentityKind::CompositionProvider,
                        provider.id.clone(),
                    ),
                    label,
                    child(&provider_base, "id"),
                );
                self.reference_composition_participant(
                    &provider.participant,
                    label,
                    &child(&provider_base, "participant"),
                );
                for (capability_index, capability) in provider.capability_ids.iter().enumerate() {
                    self.reference(
                        simple_identity(AuthoringIdentityKind::Capability, capability.clone()),
                        label,
                        nested(
                            &provider_base,
                            [field("capability_ids"), at(capability_index)],
                        ),
                    );
                }
                for (compatible_index, compatible) in
                    provider.compatible_provider_ids.iter().enumerate()
                {
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionProvider,
                            compatible.clone(),
                        ),
                        label,
                        nested(
                            &provider_base,
                            [field("compatible_provider_ids"), at(compatible_index)],
                        ),
                    );
                }
            }
        }
        for (index, binding) in declarations.provider_bindings.iter().enumerate() {
            let base = nested(&root, [field("provider_bindings"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionProviderBinding,
                    binding.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference_composition_participant(
                &binding.participant,
                label,
                &child(&base, "participant"),
            );
            for (contract_index, contract) in binding.responsibility_contract_ids.iter().enumerate()
            {
                self.reference(
                    simple_identity(
                        AuthoringIdentityKind::CompositionResponsibilityContract,
                        contract.clone(),
                    ),
                    label,
                    nested(
                        &base,
                        [field("responsibility_contract_ids"), at(contract_index)],
                    ),
                );
            }
        }
        for (index, contract) in declarations.entry_contracts.iter().enumerate() {
            let base = nested(&root, [field("entry_contracts"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionEntryContract,
                    contract.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference_composition_participant(&contract.entry, label, &child(&base, "entry"));
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::CompositionResponsibilityContract,
                    contract.responsibility_contract_id.clone(),
                ),
                label,
                child(&base, "responsibility_contract_id"),
            );
        }
        for (index, translation) in declarations.request_translations.iter().enumerate() {
            let base = nested(&root, [field("request_translations"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionRequestTranslation,
                    translation.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference_composition_participant(&translation.from, label, &child(&base, "from"));
            self.reference_composition_participant(&translation.to, label, &child(&base, "to"));
        }
        for (index, translation) in declarations.result_translations.iter().enumerate() {
            let base = nested(&root, [field("result_translations"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionResultTranslation,
                    translation.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference_composition_participant(&translation.from, label, &child(&base, "from"));
            self.reference_composition_participant(&translation.to, label, &child(&base, "to"));
        }
        for (index, contract) in declarations.termination_contracts.iter().enumerate() {
            let base = nested(&root, [field("termination_contracts"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionTerminationContract,
                    contract.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference_composition_participant(&contract.owner, label, &child(&base, "owner"));
            if let Some(terminal_leaf) = &contract.terminal_leaf {
                self.reference_composition_participant(
                    terminal_leaf,
                    label,
                    &child(&base, "terminal_leaf"),
                );
            }
        }
        for (index, contract) in declarations.settlement_contracts.iter().enumerate() {
            let base = nested(&root, [field("settlement_contracts"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionSettlementContract,
                    contract.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference_composition_participant(
                &contract.observer,
                label,
                &child(&base, "observer"),
            );
        }
        for (index, contract) in declarations.continuity_contracts.iter().enumerate() {
            let base = nested(&root, [field("continuity_contracts"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::CompositionContinuityContract,
                    contract.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference_composition_participant(&contract.owner, label, &child(&base, "owner"));
        }
        for (index, edge) in declarations.edge_contracts.iter().enumerate() {
            let base = nested(&root, [field("edge_contracts"), at(index)]);
            self.define(
                simple_identity(AuthoringIdentityKind::CompositionEdge, edge.id().to_owned()),
                label,
                child(&base, "id"),
            );
            use pale_core::records::CompositionEdgeV1;
            match edge {
                CompositionEdgeV1::WorkDelegation {
                    from,
                    to,
                    responsibility_contract_id,
                    request_translation_id,
                    result_translation_id,
                    ..
                } => {
                    self.reference_composition_participant(from, label, &child(&base, "from"));
                    self.reference_composition_participant(to, label, &child(&base, "to"));
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionResponsibilityContract,
                            responsibility_contract_id.clone(),
                        ),
                        label,
                        child(&base, "responsibility_contract_id"),
                    );
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionRequestTranslation,
                            request_translation_id.clone(),
                        ),
                        label,
                        child(&base, "request_translation_id"),
                    );
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionResultTranslation,
                            result_translation_id.clone(),
                        ),
                        label,
                        child(&base, "result_translation_id"),
                    );
                }
                CompositionEdgeV1::NativeApplicationCall {
                    caller,
                    application,
                    provider_binding_id,
                    request_translation_id,
                    result_translation_id,
                    ..
                } => {
                    self.reference_composition_participant(caller, label, &child(&base, "caller"));
                    self.reference_composition_participant(
                        application,
                        label,
                        &child(&base, "application"),
                    );
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionProviderBinding,
                            provider_binding_id.clone(),
                        ),
                        label,
                        child(&base, "provider_binding_id"),
                    );
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionRequestTranslation,
                            request_translation_id.clone(),
                        ),
                        label,
                        child(&base, "request_translation_id"),
                    );
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionResultTranslation,
                            result_translation_id.clone(),
                        ),
                        label,
                        child(&base, "result_translation_id"),
                    );
                }
                CompositionEdgeV1::TerminalLeaf {
                    owner,
                    terminal_leaf,
                    termination_contract_id,
                    ..
                } => {
                    self.reference_composition_participant(owner, label, &child(&base, "owner"));
                    self.reference_composition_participant(
                        terminal_leaf,
                        label,
                        &child(&base, "terminal_leaf"),
                    );
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionTerminationContract,
                            termination_contract_id.clone(),
                        ),
                        label,
                        child(&base, "termination_contract_id"),
                    );
                }
                CompositionEdgeV1::ObservationSettlement {
                    subject,
                    observer,
                    settlement_contract_id,
                    ..
                } => {
                    self.reference_composition_participant(
                        subject,
                        label,
                        &child(&base, "subject"),
                    );
                    self.reference_composition_participant(
                        observer,
                        label,
                        &child(&base, "observer"),
                    );
                    self.reference(
                        simple_identity(
                            AuthoringIdentityKind::CompositionSettlementContract,
                            settlement_contract_id.clone(),
                        ),
                        label,
                        child(&base, "settlement_contract_id"),
                    );
                }
                CompositionEdgeV1::UiEmbedding { host, embedded, .. } => {
                    self.reference_composition_participant(host, label, &child(&base, "host"));
                    self.reference_composition_participant(
                        embedded,
                        label,
                        &child(&base, "embedded"),
                    );
                }
                CompositionEdgeV1::PluginDependency {
                    plugin, dependency, ..
                } => {
                    self.reference_composition_participant(plugin, label, &child(&base, "plugin"));
                    self.reference_composition_participant(
                        dependency,
                        label,
                        &child(&base, "dependency"),
                    );
                }
            }
        }
        for (index, experience_path) in declarations.selected_experience_paths.iter().enumerate() {
            let base = nested(&root, [field("selected_experience_paths"), at(index)]);
            self.define(
                simple_identity(
                    AuthoringIdentityKind::ExperiencePath,
                    experience_path.id.clone(),
                ),
                label,
                child(&base, "id"),
            );
            self.reference(
                simple_identity(
                    AuthoringIdentityKind::CompositionEntryContract,
                    experience_path.entry_contract_id.clone(),
                ),
                label,
                child(&base, "entry_contract_id"),
            );
            for (edge_index, edge) in experience_path.work_delegation_edge_ids.iter().enumerate() {
                self.reference(
                    simple_identity(AuthoringIdentityKind::CompositionEdge, edge.clone()),
                    label,
                    nested(&base, [field("work_delegation_edge_ids"), at(edge_index)]),
                );
            }
            for (contract_index, contract) in experience_path
                .responsibility_contract_ids
                .iter()
                .enumerate()
            {
                self.reference(
                    simple_identity(
                        AuthoringIdentityKind::CompositionResponsibilityContract,
                        contract.clone(),
                    ),
                    label,
                    nested(
                        &base,
                        [field("responsibility_contract_ids"), at(contract_index)],
                    ),
                );
            }
            for (binding_index, binding) in experience_path.provider_binding_ids.iter().enumerate()
            {
                self.reference(
                    simple_identity(
                        AuthoringIdentityKind::CompositionProviderBinding,
                        binding.clone(),
                    ),
                    label,
                    nested(&base, [field("provider_binding_ids"), at(binding_index)]),
                );
            }
            for (capability_index, capability) in
                experience_path.capability_ceiling.iter().enumerate()
            {
                self.reference(
                    simple_identity(AuthoringIdentityKind::Capability, capability.clone()),
                    label,
                    nested(&base, [field("capability_ceiling"), at(capability_index)]),
                );
            }
            for (invocation_index, invocation) in experience_path.invocations.iter().enumerate() {
                self.reference(
                    simple_identity(
                        AuthoringIdentityKind::CompositionEdge,
                        invocation.edge_id.clone(),
                    ),
                    label,
                    nested(
                        &base,
                        [field("invocations"), at(invocation_index), field("edge_id")],
                    ),
                );
            }
            for (transition_index, transition) in
                experience_path.native_owner_transitions.iter().enumerate()
            {
                self.reference(
                    simple_identity(
                        AuthoringIdentityKind::CompositionEdge,
                        transition.edge_id.clone(),
                    ),
                    label,
                    nested(
                        &base,
                        [
                            field("native_owner_transitions"),
                            at(transition_index),
                            field("edge_id"),
                        ],
                    ),
                );
            }
            for (outcome_index, outcome) in experience_path.native_outcomes.iter().enumerate() {
                self.reference(
                    simple_identity(
                        AuthoringIdentityKind::CompositionEdge,
                        outcome.edge_id().to_owned(),
                    ),
                    label,
                    nested(
                        &base,
                        [
                            field("native_outcomes"),
                            at(outcome_index),
                            field("edge_id"),
                        ],
                    ),
                );
            }
            for (consequence_index, consequence) in
                experience_path.destination_consequences.iter().enumerate()
            {
                self.reference_composition_participant(
                    &consequence.destination,
                    label,
                    &nested(
                        &base,
                        [
                            field("destination_consequences"),
                            at(consequence_index),
                            field("destination"),
                        ],
                    ),
                );
            }
            for (field_name, kind, value) in [
                (
                    "termination_contract_id",
                    AuthoringIdentityKind::CompositionTerminationContract,
                    experience_path.termination_contract_id.as_str(),
                ),
                (
                    "settlement_contract_id",
                    AuthoringIdentityKind::CompositionSettlementContract,
                    experience_path.settlement_contract_id.as_str(),
                ),
                (
                    "continuity_contract_id",
                    AuthoringIdentityKind::CompositionContinuityContract,
                    experience_path.continuity_contract_id.as_str(),
                ),
            ] {
                self.reference(
                    simple_identity(kind, value.to_owned()),
                    label,
                    child(&base, field_name),
                );
            }
        }
    }

    fn define_composition_participant(
        &mut self,
        participant: &pale_core::records::ParticipantRefV1,
        source_label: &str,
        base: &[AuthoringInstancePathSegment],
    ) {
        let (identity, id_field) = composition_participant_identity(participant);
        self.define(identity, source_label, child(base, id_field));
    }

    fn reference_composition_participant(
        &mut self,
        participant: &pale_core::records::ParticipantRefV1,
        source_label: &str,
        base: &[AuthoringInstancePathSegment],
    ) {
        let (identity, id_field) = composition_participant_identity(participant);
        self.reference(identity, source_label, child(base, id_field));
    }

    fn index_locks(&mut self, linked: &LinkedWorkspace) {
        let resolution = &linked.source.resolution_lock;
        let resolution_label = resolution.source.source_path.as_str();
        for (index, release) in resolution.value.releases.iter().enumerate() {
            let base = path([field("releases"), at(index)]);
            let module = release.module_id.to_string();
            self.reference_denoted(
                DenotedAuthoringIdentity::module(&release.module_id),
                resolution_label,
                child(&base, "module_id"),
            );
            self.reference(
                identity(
                    AuthoringIdentityKind::ModuleRelease,
                    format!("{module}::{}", release.release_id),
                ),
                resolution_label,
                child(&base, "release_id"),
            );
        }

        let universe = &linked.source.universe_lock;
        let universe_label = universe.source.source_path.as_str();
        self.reference_denoted(
            DenotedAuthoringIdentity::workspace(&universe.value.workspace_id),
            universe_label,
            path([field("workspace_id")]),
        );
        for (index, release) in universe.value.adopted_releases.iter().enumerate() {
            let base = path([field("adopted_releases"), at(index)]);
            let module = release.module_id.to_string();
            self.reference_denoted(
                DenotedAuthoringIdentity::module(&release.module_id),
                universe_label,
                child(&base, "module_id"),
            );
            self.reference(
                identity(
                    AuthoringIdentityKind::ModuleRelease,
                    format!("{module}::{}", release.release_id),
                ),
                universe_label,
                child(&base, "release_id"),
            );
        }
    }

    fn index_components(&mut self, linked: &LinkedWorkspace) {
        for component in linked.source.components.values() {
            let label = component.source.source_path.as_str();
            let owner = component.value.module_id.to_string();
            self.reference_denoted(
                DenotedAuthoringIdentity::module(&component.value.module_id),
                label,
                path([field("module_id")]),
            );
            self.define_denoted(
                DenotedAuthoringIdentity::component(
                    &component.value.module_id,
                    &component.value.component_id,
                ),
                label,
                path([field("component_id")]),
            );
            for (declaration_index, declaration) in component.value.declarations.iter().enumerate()
            {
                let qualified = QualifiedDeclarationRef {
                    owner_module: component.value.module_id.clone(),
                    local_ref: declaration.declaration_ref(),
                };
                let base = path([field("declarations"), at(declaration_index)]);
                self.define_denoted(
                    DenotedAuthoringIdentity::declaration(&qualified),
                    label,
                    child(&base, "id"),
                );
                self.define(
                    identity(
                        AuthoringIdentityKind::Namespace,
                        format!("{owner}::{}", declaration.declaration_ref().namespace),
                    ),
                    label,
                    child(&base, "namespace"),
                );
                // Captured definitions need not belong to the selected universe.
                // Only linked declarations supply resolved reference targets.
                let Some(linked_declaration) = linked.declarations.get(&qualified) else {
                    continue;
                };
                for (dependency_index, dependency) in
                    linked_declaration.dependencies.iter().enumerate()
                {
                    self.reference_denoted(
                        DenotedAuthoringIdentity::declaration(&dependency.target),
                        label,
                        nested(
                            &base,
                            [
                                field("dependencies"),
                                at(dependency_index),
                                field("target"),
                                field("declaration_id"),
                            ],
                        ),
                    );
                }
                if let WorkspaceDeclarationV1::Relationship { .. } = declaration {
                    if let Some(target) = &linked_declaration.relationship_from {
                        self.reference_denoted(
                            DenotedAuthoringIdentity::declaration(target),
                            label,
                            nested(&base, [field("from"), field("declaration_id")]),
                        );
                    }
                    if let Some(target) = &linked_declaration.relationship_to {
                        self.reference_denoted(
                            DenotedAuthoringIdentity::declaration(target),
                            label,
                            nested(&base, [field("to"), field("declaration_id")]),
                        );
                    }
                }
            }
        }
    }

    fn index_modules(&mut self, linked: &LinkedWorkspace) {
        let access = linked
            .access_entries
            .iter()
            .map(|entry| {
                (
                    (entry.importer_module.clone(), entry.local_ref.clone()),
                    entry.canonical.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for retained in linked.source.modules.values() {
            let label = retained.source.source_path.as_str();
            let module = &retained.value.module;
            let owner = module.module_id.to_string();
            self.define_denoted(
                DenotedAuthoringIdentity::module(&module.module_id),
                label,
                path([field("module"), field("module_id")]),
            );
            self.define(
                identity(
                    AuthoringIdentityKind::ModuleRelease,
                    format!("{owner}::{}", module.release_id),
                ),
                label,
                path([field("module"), field("release_id")]),
            );
            for (source_index, source) in module.source_components.iter().enumerate() {
                self.reference_denoted(
                    DenotedAuthoringIdentity::component(&module.module_id, &source.component_id),
                    label,
                    path([
                        field("module"),
                        field("source_components"),
                        at(source_index),
                        field("component_id"),
                    ]),
                );
            }
            for (export_index, export) in module.exports.iter().enumerate() {
                let export_base = path([field("module"), field("exports"), at(export_index)]);
                self.define(
                    identity(
                        AuthoringIdentityKind::Export,
                        format!("{owner}::{}", export.export_id),
                    ),
                    label,
                    child(&export_base, "export_id"),
                );
                for (projection_index, projection) in export.declarations.iter().enumerate() {
                    let qualified = QualifiedDeclarationRef {
                        owner_module: projection.owner_module.clone(),
                        local_ref: projection.declaration.declaration_ref(),
                    };
                    let projection_base =
                        nested(&export_base, [field("declarations"), at(projection_index)]);
                    self.reference_denoted(
                        DenotedAuthoringIdentity::module(&projection.owner_module),
                        label,
                        child(&projection_base, "owner_module"),
                    );
                    if let Some(target) =
                        access.get(&(module.module_id.clone(), projection.local_ref.clone()))
                    {
                        self.reference_denoted(
                            DenotedAuthoringIdentity::declaration(target),
                            label,
                            nested(
                                &projection_base,
                                [field("local_ref"), field("declaration_id")],
                            ),
                        );
                    }
                    self.reference(
                        identity(
                            AuthoringIdentityKind::Namespace,
                            format!("{owner}::{}", projection.local_ref.namespace),
                        ),
                        label,
                        nested(&projection_base, [field("local_ref"), field("namespace")]),
                    );
                    self.insert_denoted(
                        DenotedAuthoringIdentity::declaration(&qualified),
                        label,
                        nested(&projection_base, [field("declaration"), field("id")]),
                        AuthoringOccurrenceRole::Projection,
                    );
                    for (dependency_index, dependency) in
                        projection.declaration.dependencies().iter().enumerate()
                    {
                        let target = QualifiedDeclarationRef {
                            owner_module: projection.owner_module.clone(),
                            local_ref: dependency.target.clone(),
                        };
                        self.reference_denoted(
                            DenotedAuthoringIdentity::declaration(&target),
                            label,
                            nested(
                                &projection_base,
                                [
                                    field("declaration"),
                                    field("dependencies"),
                                    at(dependency_index),
                                    field("target"),
                                    field("declaration_id"),
                                ],
                            ),
                        );
                    }
                    if let WorkspaceDeclarationV1::Relationship { from, to, .. } =
                        &projection.declaration
                    {
                        for (endpoint, field_name) in [(from, "from"), (to, "to")] {
                            let target = QualifiedDeclarationRef {
                                owner_module: projection.owner_module.clone(),
                                local_ref: endpoint.clone(),
                            };
                            self.reference_denoted(
                                DenotedAuthoringIdentity::declaration(&target),
                                label,
                                nested(
                                    &projection_base,
                                    [
                                        field("declaration"),
                                        field(field_name),
                                        field("declaration_id"),
                                    ],
                                ),
                            );
                        }
                    }
                }
            }
            for (import_index, import) in module.imports.iter().enumerate() {
                let import_base = path([field("module"), field("imports"), at(import_index)]);
                let upstream = import.module_id.to_string();
                self.reference_denoted(
                    DenotedAuthoringIdentity::module(&import.module_id),
                    label,
                    child(&import_base, "module_id"),
                );
                self.reference(
                    identity(
                        AuthoringIdentityKind::ModuleRelease,
                        format!("{upstream}::{}", import.release_id),
                    ),
                    label,
                    child(&import_base, "release_id"),
                );
                for (path_index, import_path) in import.paths.iter().enumerate() {
                    let path_base = nested(&import_base, [field("paths"), at(path_index)]);
                    self.reference(
                        identity(
                            AuthoringIdentityKind::Export,
                            format!("{upstream}::{}", import_path.export_id),
                        ),
                        label,
                        child(&path_base, "export_id"),
                    );
                    self.reference(
                        identity(
                            AuthoringIdentityKind::Namespace,
                            format!("{upstream}::{}", import_path.source_namespace),
                        ),
                        label,
                        child(&path_base, "source_namespace"),
                    );
                    self.define(
                        identity(
                            AuthoringIdentityKind::Namespace,
                            format!("{owner}::{}", import_path.local_namespace),
                        ),
                        label,
                        child(&path_base, "local_namespace"),
                    );
                }
                if let Some(resource) = &import.protected_delivery_authority {
                    self.reference_denoted(
                        DenotedAuthoringIdentity::resource(&module.module_id, resource),
                        label,
                        child(&import_base, "protected_delivery_authority"),
                    );
                }
            }
            for (resource_index, resource) in module.resources.iter().enumerate() {
                let resource_base = path([field("module"), field("resources"), at(resource_index)]);
                self.define_denoted(
                    DenotedAuthoringIdentity::resource(&module.module_id, resource.resource_id()),
                    label,
                    child(&resource_base, "resource_id"),
                );
                if let Some(resolver) = resource.authority_resolver() {
                    self.reference_denoted(
                        DenotedAuthoringIdentity::resource(&module.module_id, resolver),
                        label,
                        child(&resource_base, "authority_resolver"),
                    );
                }
            }
            for (extension_index, extension) in module.extensions.iter().enumerate() {
                let extension_base =
                    path([field("module"), field("extensions"), at(extension_index)]);
                self.reference(
                    identity(
                        AuthoringIdentityKind::Namespace,
                        format!("{owner}::{}", extension.namespace),
                    ),
                    label,
                    child(&extension_base, "namespace"),
                );
                if let Some(target) =
                    access.get(&(module.module_id.clone(), extension.target.clone()))
                {
                    self.reference_denoted(
                        DenotedAuthoringIdentity::declaration(target),
                        label,
                        nested(&extension_base, [field("target"), field("declaration_id")]),
                    );
                }
                for (trait_index, trait_ref) in extension.added_traits.iter().enumerate() {
                    if let Some(target) = access.get(&(module.module_id.clone(), trait_ref.clone()))
                    {
                        self.reference_denoted(
                            DenotedAuthoringIdentity::declaration(target),
                            label,
                            nested(
                                &extension_base,
                                [
                                    field("added_traits"),
                                    at(trait_index),
                                    field("declaration_id"),
                                ],
                            ),
                        );
                    }
                }
            }
        }
    }

    fn define(
        &mut self,
        identity: AuthoringIdentity,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
    ) {
        self.insert(
            identity,
            source_label,
            path,
            AuthoringOccurrenceRole::Definition,
        );
    }

    fn define_denoted(
        &mut self,
        denotation: DenotedAuthoringIdentity,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
    ) {
        self.insert_denoted(
            denotation,
            source_label,
            path,
            AuthoringOccurrenceRole::Definition,
        );
    }

    fn reference_denoted(
        &mut self,
        denotation: DenotedAuthoringIdentity,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
    ) {
        self.insert_denoted(
            denotation,
            source_label,
            path,
            AuthoringOccurrenceRole::Reference,
        );
    }

    fn reference(
        &mut self,
        identity: AuthoringIdentity,
        source_label: &str,
        path: Vec<AuthoringInstancePathSegment>,
    ) {
        self.insert(
            identity,
            source_label,
            path,
            AuthoringOccurrenceRole::Reference,
        );
    }
}

fn normalized_occurrence_page_limit(requested: usize) -> usize {
    if requested == 0 {
        DEFAULT_AUTHORING_OCCURRENCE_PAGE_LIMIT
    } else {
        requested.min(MAX_AUTHORING_OCCURRENCE_PAGE_LIMIT)
    }
}

impl AuthoringCompletionDomain {
    fn from_identity(identity: &AuthoringIdentity) -> Self {
        Self {
            kind: identity.kind,
            completion_scope: identity.completion_scope.clone(),
        }
    }

    fn matches_identity(&self, identity: &AuthoringIdentity) -> bool {
        self.kind == identity.kind && self.completion_scope == identity.completion_scope
    }
}

fn identity(kind: AuthoringIdentityKind, value: String) -> AuthoringIdentity {
    let (completion_scope, authored_value) = if kind == AuthoringIdentityKind::Module {
        (String::new(), value.clone())
    } else {
        value.rsplit_once("::").map_or_else(
            || (String::new(), value.clone()),
            |(scope, authored)| (scope.to_owned(), authored.to_owned()),
        )
    };
    AuthoringIdentity {
        kind,
        value,
        completion_scope,
        authored_value,
    }
}

fn simple_identity(kind: AuthoringIdentityKind, value: String) -> AuthoringIdentity {
    AuthoringIdentity {
        kind,
        authored_value: value.clone(),
        value,
        completion_scope: String::new(),
    }
}

fn source_document_identity(label: &str, grammar: &str) -> AuthoringIdentity {
    scoped_identity(
        AuthoringIdentityKind::SourceDocument,
        label.to_owned(),
        grammar.to_owned(),
        label.to_owned(),
    )
}

fn qualified_identity(
    kind: AuthoringIdentityKind,
    namespace: impl std::fmt::Display,
    local: impl std::fmt::Display,
) -> AuthoringIdentity {
    let namespace = namespace.to_string();
    let local = local.to_string();
    scoped_identity(kind, format!("{namespace}::{local}"), namespace, local)
}

fn scope_identity(value: &pale_core::records::LogicalScopeRefV1) -> AuthoringIdentity {
    qualified_identity(
        AuthoringIdentityKind::LogicalScope,
        &value.namespace,
        &value.scope_id,
    )
}

/// Reuse the indexing owner's exact named-identity recipes for inspected records.
/// Complete bindings/derived edges without their own indexed name stay unmapped.
pub(super) fn organization_record_identity(
    record: super::OrganizationInspectionRecord<'_>,
) -> Option<AuthoringIdentity> {
    use super::OrganizationInspectionRecord as Row;
    Some(match record {
        Row::Scope(v) => scope_identity(&v.scope),
        Row::Organization(v) => organization_identity(&v.organization),
        Row::Group(v) => group_identity(&v.group),
        Row::DisciplineTree(v) => discipline_tree_identity(&v.tree),
        Row::Role(v) => organizational_role_identity(&v.role),
        Row::Leadership(v) => {
            simple_identity(AuthoringIdentityKind::LeadershipTopic, v.topic.to_string())
        }
        Row::Relation(v) => simple_identity(
            AuthoringIdentityKind::OrganizationalRelation,
            v.relation_id.to_string(),
        ),
        Row::Object(v) => simple_identity(
            AuthoringIdentityKind::OrganizationalObject,
            v.object_id.to_string(),
        ),
        Row::GroupLens(v) => simple_identity(
            AuthoringIdentityKind::GroupLensProfile,
            v.profile_id.to_string(),
        ),
        Row::Service(v) => directional_service_identity(&v.contract_ref()),
        Row::NativeProjection(v) => simple_identity(
            AuthoringIdentityKind::OrganizationalProjection,
            v.projection_id.to_string(),
        ),
        Row::Inspection(v) => simple_identity(
            AuthoringIdentityKind::OrganizationalInspection,
            v.inspection_id.to_string(),
        ),
        Row::Responsibility(v) => simple_identity(
            AuthoringIdentityKind::ResponsibilityDerivation,
            v.derivation_id.to_string(),
        ),
        Row::OccupantBinding(_)
        | Row::ProviderBinding(_)
        | Row::CommunicationActor(_)
        | Row::PopulationRule(_)
        | Row::DerivedRelation(_) => return None,
    })
}

fn organization_identity(
    value: &pale_core::records::LogicalOrganizationRefV1,
) -> AuthoringIdentity {
    qualified_identity(
        AuthoringIdentityKind::LogicalOrganization,
        &value.namespace,
        &value.organization_id,
    )
}

fn group_identity(value: &pale_core::records::OrganizationalGroupRefV1) -> AuthoringIdentity {
    qualified_identity(
        AuthoringIdentityKind::OrganizationalGroup,
        &value.namespace,
        &value.group_id,
    )
}

fn discipline_tree_identity(value: &pale_core::records::DisciplineTreeRefV1) -> AuthoringIdentity {
    qualified_identity(
        AuthoringIdentityKind::DisciplineTree,
        &value.namespace,
        &value.tree_id,
    )
}

fn organizational_role_identity(
    value: &pale_core::records::OrganizationalRoleRefV1,
) -> AuthoringIdentity {
    qualified_identity(
        AuthoringIdentityKind::OrganizationalRole,
        &value.namespace,
        &value.role_id,
    )
}

fn directional_service_identity(
    value: &pale_core::records::DirectionalServiceContractRefV1,
) -> AuthoringIdentity {
    scoped_identity(
        AuthoringIdentityKind::DirectionalService,
        value.to_string(),
        value.namespace.to_string(),
        value.contract_id.to_string(),
    )
}

fn composition_participant_identity(
    participant: &pale_core::records::ParticipantRefV1,
) -> (AuthoringIdentity, &'static str) {
    use pale_core::records::ParticipantRefV1;

    let (participant_kind, id_field) = match participant {
        ParticipantRefV1::Interface { .. } => ("interface", "interface_id"),
        ParticipantRefV1::Application { .. } => ("application", "application_id"),
        ParticipantRefV1::Engine { .. } => ("engine", "engine_id"),
        ParticipantRefV1::Capability { .. } => ("capability", "capability_id"),
        ParticipantRefV1::TerminalLeaf { .. } => ("terminal-leaf", "terminal_leaf_id"),
        ParticipantRefV1::Plugin { .. } => ("plugin", "plugin_id"),
        ParticipantRefV1::Skill { .. } => ("skill", "skill_id"),
    };
    let participant_id = crate::composition::participant_id(participant);
    (
        scoped_identity(
            AuthoringIdentityKind::CompositionParticipant,
            format!("{participant_kind}:{participant_id}"),
            participant_kind.to_owned(),
            participant_id.to_owned(),
        ),
        id_field,
    )
}

fn scoped_identity(
    kind: AuthoringIdentityKind,
    value: String,
    completion_scope: String,
    authored_value: String,
) -> AuthoringIdentity {
    AuthoringIdentity {
        kind,
        value,
        completion_scope,
        authored_value,
    }
}

fn package_resource_identity(
    value: &pale_core::records::PackageResourceRefV1,
    kind: AuthoringIdentityKind,
) -> AuthoringIdentity {
    scoped_identity(
        kind,
        value.to_string(),
        value.package_id.to_string(),
        value.resource_id.to_string(),
    )
}

const fn package_identity_kind(
    kind: pale_core::records::PackageResourceKindV1,
) -> AuthoringIdentityKind {
    match kind {
        pale_core::records::PackageResourceKindV1::Doctrine => {
            AuthoringIdentityKind::PackageDoctrine
        }
        pale_core::records::PackageResourceKindV1::OperationArchetype => {
            AuthoringIdentityKind::PackageOperationArchetype
        }
        pale_core::records::PackageResourceKindV1::Operation => {
            AuthoringIdentityKind::PackageOperation
        }
        pale_core::records::PackageResourceKindV1::RoleArchetype => {
            AuthoringIdentityKind::PackageRoleArchetype
        }
        pale_core::records::PackageResourceKindV1::Role => AuthoringIdentityKind::PackageRole,
        pale_core::records::PackageResourceKindV1::Stack => AuthoringIdentityKind::PackageStack,
        pale_core::records::PackageResourceKindV1::RunMode => AuthoringIdentityKind::PackageRunMode,
        pale_core::records::PackageResourceKindV1::PolicyFamily => {
            AuthoringIdentityKind::PackagePolicyFamily
        }
        pale_core::records::PackageResourceKindV1::GoverningOrder => {
            AuthoringIdentityKind::PackageGoverningOrder
        }
        pale_core::records::PackageResourceKindV1::CapabilityPolicy => {
            AuthoringIdentityKind::PackageCapabilityPolicy
        }
        pale_core::records::PackageResourceKindV1::ToolRecipe => {
            AuthoringIdentityKind::PackageToolRecipe
        }
    }
}

fn module_identity(value: &str) -> AuthoringIdentity {
    simple_identity(AuthoringIdentityKind::Module, value.to_owned())
}

fn declaration_identity(value: &QualifiedDeclarationRef) -> AuthoringIdentity {
    identity(AuthoringIdentityKind::Declaration, value.to_string())
}

fn field(value: &str) -> AuthoringInstancePathSegment {
    AuthoringInstancePathSegment::Field(value.to_owned())
}

fn location_copied_string_bytes(
    source_label: &str,
    path: &[AuthoringInstancePathSegment],
) -> usize {
    path.iter()
        .filter_map(|segment| match segment {
            AuthoringInstancePathSegment::Field(field) => Some(field.len()),
            AuthoringInstancePathSegment::Index(_) => None,
        })
        .try_fold(source_label.len(), usize::checked_add)
        .expect("authoring location copied-string accounting overflowed")
}

const fn at(index: usize) -> AuthoringInstancePathSegment {
    AuthoringInstancePathSegment::Index(index)
}

fn path<const N: usize>(
    segments: [AuthoringInstancePathSegment; N],
) -> Vec<AuthoringInstancePathSegment> {
    segments.into()
}

fn child(
    path: &[AuthoringInstancePathSegment],
    field_name: &str,
) -> Vec<AuthoringInstancePathSegment> {
    nested(path, [field(field_name)])
}

fn nested<const N: usize>(
    path: &[AuthoringInstancePathSegment],
    tail: [AuthoringInstancePathSegment; N],
) -> Vec<AuthoringInstancePathSegment> {
    path.iter().cloned().chain(tail).collect()
}

fn structural_authoring_path(path: &[AuthoringInstancePathSegment]) -> Vec<AuthoringPathSegment> {
    path.iter()
        .map(|segment| match segment {
            AuthoringInstancePathSegment::Field(field) => {
                AuthoringPathSegment::Field(field.clone())
            }
            AuthoringInstancePathSegment::Index(_) => AuthoringPathSegment::ArrayItem,
        })
        .collect()
}

impl AuthoringSourceMap {
    /// Build the complete portable-workspace grammar selection.
    pub(crate) fn from_workspace(source: &ParsedWorkspaceSource) -> Result<Self, WorkspaceError> {
        let mut map = Self::default();
        map.insert(
            source.manifest_source.source_path.as_str(),
            AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("pale-workspace.v1")),
        )
        .map_err(WorkspaceError::DuplicateCapturedSource)?;
        for retained in source.modules.values() {
            map.insert(
                retained.source.source_path.as_str(),
                AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("workspace-module.v1")),
            )
            .map_err(WorkspaceError::DuplicateCapturedSource)?;
        }
        for retained in source.components.values() {
            map.insert(
                retained.source.source_path.as_str(),
                AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record(
                    "workspace-language-primitives.v1",
                )),
            )
            .map_err(WorkspaceError::DuplicateCapturedSource)?;
        }
        for retained in source.responsibility_derivations.values() {
            map.insert(
                retained.source.source_path.as_str(),
                AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record(
                    "responsibility-derivation.v1",
                )),
            )
            .map_err(WorkspaceError::DuplicateCapturedSource)?;
        }
        if let Some(retained) = &source.organizational_translation {
            map.insert(
                retained.source.source_path.as_str(),
                AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record(
                    "organizational-translation.v1",
                )),
            )
            .map_err(WorkspaceError::DuplicateCapturedSource)?;
        }
        if let Some(retained) = &source.experience_composition {
            map.insert(
                retained.source.source_path.as_str(),
                AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record(
                    "workspace-composition-source.v1",
                )),
            )
            .map_err(WorkspaceError::DuplicateCapturedSource)?;
        }
        map.insert(
            source.resolution_lock.source.source_path.as_str(),
            AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record(
                "workspace-resolution-lock.v1",
            )),
        )
        .map_err(WorkspaceError::DuplicateCapturedSource)?;
        map.insert(
            source.universe_lock.source.source_path.as_str(),
            AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("workspace-universe-lock.v1")),
        )
        .map_err(WorkspaceError::DuplicateCapturedSource)?;
        Ok(map)
    }

    /// Insert one exact logical label and reject compiler-internal ambiguity.
    pub(crate) fn insert(
        &mut self,
        label: impl Into<String>,
        kind: AuthoringDocumentKind,
    ) -> Result<(), String> {
        let label = label.into();
        match self.documents.entry(label) {
            Entry::Vacant(entry) => {
                entry.insert(kind);
                Ok(())
            }
            Entry::Occupied(entry) => Err(entry.key().clone()),
        }
    }

    /// Merge disjoint admitted source families.
    pub(crate) fn extend(&mut self, other: Self) -> Result<(), String> {
        if let Some(label) = other
            .documents
            .keys()
            .find(|label| self.documents.contains_key(*label))
        {
            return Err(label.clone());
        }
        self.documents.extend(other.documents);
        Ok(())
    }

    /// Borrow the exact compiler-selected treatment for one logical label.
    #[must_use]
    pub fn document(&self, label: &str) -> Option<AuthoringDocumentKind> {
        self.documents.get(label).copied()
    }

    /// Iterate all admitted source labels in deterministic order.
    pub fn documents(&self) -> impl ExactSizeIterator<Item = (&str, AuthoringDocumentKind)> {
        self.documents
            .iter()
            .map(|(label, kind)| (label.as_str(), *kind))
    }
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/authoring.rs"]
mod tests;
