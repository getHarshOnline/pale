//! Owned causal comparison over admitted portable workspace handles.

use std::sync::Arc;

use pale_application::workspace::{
    PortableWorkspaceDerivation, PortableWorkspaceDerivationExplanation,
    PortableWorkspaceExplanationFrontier as ApplicationExplanationFrontier,
    PortableWorkspaceSubjectComparison as ApplicationSubjectComparison, PortableWorkspaceUniverse,
    PortableWorkspaceValueComparison as ApplicationValueComparison,
};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    InvalidWorkspaceLanguageId, RecordSchemaIdentity, WorkspaceComponentId, WorkspaceDeclarationId,
    WorkspaceDeclarationRefV1, WorkspaceDerivationImpactStateV1, WorkspaceDerivationImpactV1,
    WorkspaceDerivationNodeV1, WorkspaceDerivationOriginV1, WorkspaceDerivationOutputV1,
    WorkspaceDerivationRootChangeV1, WorkspaceDerivationRootV1, WorkspaceDerivationRuleV1,
    WorkspaceDerivationSourceId, WorkspaceDerivationSubjectV1, WorkspaceExportId, WorkspaceId,
    WorkspaceModuleId, WorkspaceNamespaceId, WorkspaceReleaseId, WorkspaceResourceId,
};

use super::{portable_summary_failure, workspace_failure, PortableWorkspaceCompilation};
use crate::error::FailureSpecification;
use crate::{ApiError, PaleApi};

/// Default number of proof projections returned by one page query.
const DEFAULT_PROOF_PAGE_SIZE: u16 = 64;
/// Hard ceiling for one page of proof projections.
const MAX_PROOF_PAGE_SIZE: u16 = 256;

/// Opaque, verified comparison of two admitted portable workspaces.
///
/// This handle retains both immutable semantic owners and the exact derivation
/// receipt rebuilt from them. It is not a resident generation diff and carries
/// no runtime state, provider observation, native consequence, or effect
/// authority. Root and impact queries are paged; causal explanation returns one
/// deterministic witness from the receipt's existing proof DAG.
///
/// Let `I` be every impact subject, `A` the affected subjects, and `U` the
/// unaffected subjects. They obey the
/// disjoint-partition and cardinality laws:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/workspace-impact-partition.display.mathml"
))]
///
/// Root selection minimizes `(change-kind rank, root digest)`. The rendered
/// order assigns ranks `Removed`, then `Modified`, then `Added`, so predecessor
/// discontinuities cannot be hidden by current-frontier causes. For that root,
/// witness selection minimizes `(path length, complete digest sequence)`.
pub struct PortableWorkspaceComparison {
    pub(super) prior: Arc<PortableWorkspaceUniverse>,
    pub(super) current: Arc<PortableWorkspaceUniverse>,
    pub(super) derivation: PortableWorkspaceDerivation,
    pub(super) diagnostic_identity: RecordSchemaIdentity,
    summary: PortableWorkspaceComparisonSummary,
}

/// Exact input custody shared by bounded queries over an existing comparison.
#[derive(Clone)]
pub(super) struct ComparisonInputs {
    prior: Arc<PortableWorkspaceUniverse>,
    current: Arc<PortableWorkspaceUniverse>,
    receipt_digest: String,
}

impl ComparisonInputs {
    pub(super) fn retain(comparison: &PortableWorkspaceComparison) -> Self {
        Self {
            prior: Arc::clone(&comparison.prior),
            current: Arc::clone(&comparison.current),
            receipt_digest: comparison.receipt_digest().to_owned(),
        }
    }

    pub(super) fn matches(&self, comparison: &PortableWorkspaceComparison) -> bool {
        Arc::ptr_eq(&self.prior, &comparison.prior)
            && Arc::ptr_eq(&self.current, &comparison.current)
            && self.receipt_digest == comparison.receipt_digest()
    }
}

/// Owned identity-and-count projection of one verified workspace comparison.
///
/// This value preserves every public scalar observation on
/// [`PortableWorkspaceComparison`] without retaining either admitted semantic
/// owner or the causal proof DAG. Use the comparison handle itself for paging
/// and [`PortableWorkspaceComparison::explain`].
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct PortableWorkspaceComparisonSummary {
    prior_principal_universe_digest: String,
    current_principal_universe_digest: String,
    prior_semantic_ir_digest: String,
    current_semantic_ir_digest: String,
    receipt_digest: String,
    change_root_count: u32,
    affected_count: u32,
    unaffected_count: u32,
    /// Exact API-owned JCS projection of every preceding field.
    #[serde(skip)]
    #[cfg_attr(feature = "wire-schema", schemars(skip))]
    canonical_bytes: Vec<u8>,
}

impl PortableWorkspaceComparisonSummary {
    fn project(
        prior: &PortableWorkspaceUniverse,
        current: &PortableWorkspaceUniverse,
        derivation: &PortableWorkspaceDerivation,
    ) -> Result<Self, pale_core::canon::CanonError> {
        let mut summary = Self {
            prior_principal_universe_digest: prior.principal_universe_digest().to_owned(),
            current_principal_universe_digest: current.principal_universe_digest().to_owned(),
            prior_semantic_ir_digest: prior.semantic_ir_digest().to_owned(),
            current_semantic_ir_digest: current.semantic_ir_digest().to_owned(),
            receipt_digest: derivation.receipt().receipt_digest.to_string(),
            change_root_count: derivation.root_count(),
            affected_count: derivation.affected_count(),
            unaffected_count: derivation.unaffected_count(),
            canonical_bytes: Vec::new(),
        };
        summary.canonical_bytes = pale_core::canon::to_canonical_string(&summary)?.into_bytes();
        Ok(summary)
    }

    /// Exact canonical JSON projection of this complete summary.
    ///
    /// The API constructs these bytes from the same private fields exposed by
    /// the typed accessors, so consumer leaves need no parallel JSON model.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Principal-relative identity of the predecessor admitted universe.
    #[must_use]
    pub fn prior_principal_universe_digest(&self) -> &str {
        &self.prior_principal_universe_digest
    }

    /// Principal-relative identity of the current admitted universe.
    #[must_use]
    pub fn current_principal_universe_digest(&self) -> &str {
        &self.current_principal_universe_digest
    }

    /// Verified predecessor semantic-IR identity.
    #[must_use]
    pub fn prior_semantic_ir_digest(&self) -> &str {
        &self.prior_semantic_ir_digest
    }

    /// Verified current semantic-IR identity.
    #[must_use]
    pub fn current_semantic_ir_digest(&self) -> &str {
        &self.current_semantic_ir_digest
    }

    /// Domain-separated identity of the exact derivation receipt.
    #[must_use]
    pub fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }

    /// Number of exact source-change roots.
    #[must_use]
    pub const fn change_root_count(&self) -> u32 {
        self.change_root_count
    }

    /// Number of current subjects reachable from one or more change roots.
    #[must_use]
    pub const fn affected_count(&self) -> u32 {
        self.affected_count
    }

    /// Number of current subjects outside every change-root closure.
    #[must_use]
    pub const fn unaffected_count(&self) -> u32 {
        self.unaffected_count
    }
}

impl std::fmt::Debug for PortableWorkspaceComparison {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceComparison")
            .field(
                "prior_principal_universe",
                &self.prior.principal_universe_digest(),
            )
            .field(
                "current_principal_universe",
                &self.current.principal_universe_digest(),
            )
            .field("receipt_digest", &self.derivation.receipt().receipt_digest)
            .field("root_count", &self.derivation.root_count())
            .field("affected_count", &self.derivation.affected_count())
            .field("unaffected_count", &self.derivation.unaffected_count())
            .finish_non_exhaustive()
    }
}

impl PortableWorkspaceComparison {
    /// Default entries returned when a page query receives a zero limit.
    pub const DEFAULT_PAGE_SIZE: u16 = DEFAULT_PROOF_PAGE_SIZE;
    /// Maximum entries returned by [`Self::change_roots`] or [`Self::impacts`].
    pub const MAX_PAGE_SIZE: u16 = MAX_PROOF_PAGE_SIZE;

    /// Retain an application-owned comparison without reconstructing it.
    pub(crate) fn from_application_parts(
        prior: Arc<PortableWorkspaceUniverse>,
        current: Arc<PortableWorkspaceUniverse>,
        derivation: PortableWorkspaceDerivation,
        diagnostic_identity: RecordSchemaIdentity,
    ) -> Result<Self, pale_core::canon::CanonError> {
        let summary = PortableWorkspaceComparisonSummary::project(&prior, &current, &derivation)?;
        Ok(Self {
            prior,
            current,
            derivation,
            diagnostic_identity,
            summary,
        })
    }

    /// Project every public comparison identity and count into an owned value.
    ///
    /// The returned projection carries no proof graph and therefore cannot
    /// answer [`Self::change_roots`], [`Self::impacts`], or [`Self::explain`].
    #[must_use]
    pub fn summary(&self) -> PortableWorkspaceComparisonSummary {
        self.summary.clone()
    }

    /// Test whether this comparison retains these exact ordered compilations.
    ///
    /// Equal digests are not enough: separately admitted compilations can have
    /// equal public identities while retaining different source custody. This
    /// check compares the opaque semantic owners and grants no disclosure or
    /// runtime authority.
    #[must_use]
    pub fn compares(
        &self,
        prior: &PortableWorkspaceCompilation,
        current: &PortableWorkspaceCompilation,
    ) -> bool {
        Arc::ptr_eq(&self.prior, &prior.universe) && Arc::ptr_eq(&self.current, &current.universe)
    }

    /// Principal-relative identity of the predecessor admitted universe.
    #[must_use]
    pub fn prior_principal_universe_digest(&self) -> &str {
        self.prior.principal_universe_digest()
    }

    /// Principal-relative identity of the current admitted universe.
    #[must_use]
    pub fn current_principal_universe_digest(&self) -> &str {
        self.current.principal_universe_digest()
    }

    /// Verified predecessor semantic-IR identity.
    #[must_use]
    pub fn prior_semantic_ir_digest(&self) -> &str {
        self.prior.semantic_ir_digest()
    }

    /// Verified current semantic-IR identity.
    #[must_use]
    pub fn current_semantic_ir_digest(&self) -> &str {
        self.current.semantic_ir_digest()
    }

    /// Domain-separated identity of the exact derivation receipt.
    #[must_use]
    pub fn receipt_digest(&self) -> &str {
        self.derivation.receipt().receipt_digest.as_str()
    }

    /// Number of exact source-change roots.
    #[must_use]
    pub const fn change_root_count(&self) -> u32 {
        self.derivation.root_count()
    }

    /// Number of current subjects reachable from one or more change roots.
    #[must_use]
    pub const fn affected_count(&self) -> u32 {
        self.derivation.affected_count()
    }

    /// Number of current subjects outside every change-root closure.
    #[must_use]
    pub const fn unaffected_count(&self) -> u32 {
        self.derivation.unaffected_count()
    }

    /// Project one bounded page of exact source-change roots.
    ///
    /// `offset` is a zero-based entry offset. A zero `limit` selects
    /// [`Self::DEFAULT_PAGE_SIZE`] entries; larger limits are capped at
    /// [`Self::MAX_PAGE_SIZE`]. An offset at or beyond the end returns an empty
    /// terminal page.
    #[must_use]
    pub fn change_roots(&self, offset: u32, limit: u16) -> PortableWorkspaceChangeRootPage {
        let window = PageWindow::new(self.change_root_count(), offset, limit);
        let entries = self.derivation.receipt().roots[window.range()]
            .iter()
            .map(|root| {
                PortableWorkspaceChangeRoot::from_internal(root, &self.derivation.receipt().nodes)
            })
            .collect();
        PortableWorkspaceChangeRootPage::new(entries, window.next_offset)
    }

    /// Project one bounded page of the complete current impact partition.
    ///
    /// `Affected` means graph reachability from a change root; it does not by
    /// itself assert that the target's typed output differs. Paging follows the
    /// canonical subject order sealed by the receipt and uses the offset,
    /// default, cap, and terminal rules documented by [`Self::change_roots`].
    #[must_use]
    pub fn impacts(&self, offset: u32, limit: u16) -> PortableWorkspaceImpactPage {
        let window = PageWindow::new(self.derivation.impact_count(), offset, limit);
        let entries = self.derivation.receipt().impacts[window.range()]
            .iter()
            .map(PortableWorkspaceImpact::from_internal)
            .collect();
        PortableWorkspaceImpactPage::new(entries, window.next_offset)
    }

    /// Compare one subject's presence and typed value in the retained pair.
    ///
    /// The application reads its existing fact indexes and compares the typed
    /// outputs. Dependency impact remains available separately through
    /// [`Self::impacts`] and [`Self::explain`]. An affected subject can be equal.
    /// No graph traversal, source parsing or native observation occurs here.
    /// Work includes two indexed lookups, key and value comparison, and copying
    /// and encoding the selected subject and up to two outputs.
    #[must_use]
    pub fn compare_subject(
        &self,
        subject: &PortableWorkspaceSubject,
    ) -> PortableWorkspaceSubjectComparison {
        let values = match self.current.compare_subject(&self.prior, &subject.inner) {
            ApplicationSubjectComparison::Absent => PortableWorkspaceSubjectValues::Absent,
            ApplicationSubjectComparison::PriorOnly { prior } => {
                PortableWorkspaceSubjectValues::PriorOnly {
                    prior: PortableWorkspaceFact::from_internal(prior),
                }
            }
            ApplicationSubjectComparison::CurrentOnly { current } => {
                PortableWorkspaceSubjectValues::CurrentOnly {
                    current: PortableWorkspaceFact::from_internal(current),
                }
            }
            ApplicationSubjectComparison::Shared {
                prior,
                current,
                comparison,
            } => PortableWorkspaceSubjectValues::Shared {
                prior: PortableWorkspaceFact::from_internal(prior),
                current: PortableWorkspaceFact::from_internal(current),
                comparison: match comparison {
                    ApplicationValueComparison::Equal => PortableWorkspaceValueComparison::Equal,
                    ApplicationValueComparison::Changed => {
                        PortableWorkspaceValueComparison::Changed
                    }
                },
            },
        };
        PortableWorkspaceSubjectComparison::new(
            self.receipt_digest().to_owned(),
            subject.clone(),
            values,
        )
    }

    /// Validate a closed subject selector and compare its retained values.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when a selector field violates its identity grammar.
    /// A valid subject absent from both compilations is a successful result.
    pub fn compare_subject_selector(
        &self,
        selector: PortableWorkspaceSubjectSelector,
    ) -> Result<PortableWorkspaceSubjectComparison, ApiError> {
        let subject = PortableWorkspaceSubject::try_from(selector)
            .map_err(|error| selector_failure(&self.diagnostic_identity, error))?;
        Ok(self.compare_subject(&subject))
    }

    /// Explain one subject previously projected by this or another comparison.
    ///
    /// For an affected subject, the result contains one shortest root-to-target
    /// path. Root class order is rendered on [`Self`], then root digest, so a
    /// predecessor-only discontinuity is not hidden. Equal-length paths use
    /// their complete node-digest sequence. The result never pretends that a
    /// predecessor-only node exists in the current DAG, and other causal roots
    /// may also reach the target.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] only if the already-admitted proof contradicts its
    /// exact comparison owners or causal invariants.
    pub fn explain(
        &self,
        subject: &PortableWorkspaceSubject,
    ) -> Result<PortableWorkspaceExplanation, ApiError> {
        self.derivation
            .explain(&self.prior, &self.current, &subject.inner)
            .map(|explanation| {
                project_explanation(subject, explanation, &self.derivation.receipt().nodes)
            })
            .map_err(|error| workspace_failure(&self.diagnostic_identity, error))
    }

    /// Admit one transport-neutral selector and explain its causal subject.
    ///
    /// Leaves pass the closed selector unchanged so this API boundary remains
    /// the sole owner of compound-identity admission and its diagnostic.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when a selector member violates the workspace
    /// identity grammar or when the admitted proof contradicts its owners.
    pub fn explain_selector(
        &self,
        selector: PortableWorkspaceSubjectSelector,
    ) -> Result<PortableWorkspaceExplanation, ApiError> {
        let subject = PortableWorkspaceSubject::try_from(selector)
            .map_err(|error| selector_failure(&self.diagnostic_identity, error))?;
        self.explain(&subject)
    }
}

fn selector_failure(
    identity: &RecordSchemaIdentity,
    error: InvalidPortableWorkspaceSubjectSelector,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "workspace_subject_rejected",
            stage: DiagnosticStageV1::SemanticAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary: "a portable workspace subject selector is malformed",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

fn project_explanation(
    subject: &PortableWorkspaceSubject,
    explanation: PortableWorkspaceDerivationExplanation,
    current_nodes: &[WorkspaceDerivationNodeV1],
) -> PortableWorkspaceExplanation {
    match explanation {
        PortableWorkspaceDerivationExplanation::NotPresent => {
            PortableWorkspaceExplanation::not_present(subject.clone())
        }
        PortableWorkspaceDerivationExplanation::Unaffected => {
            PortableWorkspaceExplanation::unaffected(subject.clone())
        }
        PortableWorkspaceDerivationExplanation::Affected {
            root,
            frontier,
            path,
        } => PortableWorkspaceExplanation::affected(
            subject.clone(),
            PortableWorkspaceCausalExplanation {
                root: PortableWorkspaceChangeRoot::from_internal(&root, current_nodes),
                frontier: match frontier {
                    ApplicationExplanationFrontier::Predecessor => {
                        PortableWorkspaceExplanationFrontier::Predecessor
                    }
                    ApplicationExplanationFrontier::Current => {
                        PortableWorkspaceExplanationFrontier::Current
                    }
                },
                steps: path
                    .iter()
                    .map(PortableWorkspaceCausalStep::from_internal)
                    .collect(),
            },
        ),
    }
}

impl PaleApi {
    /// Compare two admitted portable workspace handles without native effects.
    ///
    /// The application owner reconstructs and verifies the exact initial-v1
    /// derivation receipt from `prior` and `current`. The façade then retains
    /// those owners behind an opaque handle so later paging and explanation do
    /// not copy source or invent a second semantic graph.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] if the two admitted owners contradict the bounded
    /// causal receipt, content-addressing, or impact-partition laws.
    pub fn compare_portable_workspaces(
        &self,
        prior: &PortableWorkspaceCompilation,
        current: &PortableWorkspaceCompilation,
    ) -> Result<PortableWorkspaceComparison, ApiError> {
        let derivation = current
            .universe
            .compare_with_predecessor(&prior.universe)
            .map_err(|error| workspace_failure(&self.diagnostic_identity, error))?;
        let summary = PortableWorkspaceComparisonSummary::project(
            &prior.universe,
            &current.universe,
            &derivation,
        )
        .map_err(|error| portable_summary_failure(&self.diagnostic_identity, error))?;
        Ok(PortableWorkspaceComparison {
            prior: Arc::clone(&prior.universe),
            current: Arc::clone(&current.universe),
            derivation,
            diagnostic_identity: self.diagnostic_identity.clone(),
            summary,
        })
    }
}

/// Closed kind of one portable causal subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableWorkspaceSubjectKind {
    /// Retained logical source document.
    Source,
    /// Admitted semantic source component.
    Component,
    /// Selected module release.
    Module,
    /// Canonically owned semantic declaration.
    Declaration,
    /// Resolved import path.
    Import,
    /// Applied extension.
    Extension,
    /// Effective declaration policy.
    Policy,
    /// Complete responsibility closure.
    ResponsibilityClosure,
    /// Complete optional organizational translation closure.
    OrganizationalTranslationClosure,
    /// Complete explicit experience-composition declaration closure.
    ExperienceComposition,
    /// Selected logical or release-bound resource.
    Resource,
    /// Exact resolution lock.
    ResolutionLock,
    /// Exact universe lock.
    UniverseLock,
    /// Complete workspace semantic IR.
    Workspace,
}

/// Owned, closed selector for one portable causal subject.
///
/// Compound coordinates keep each typed identity in a distinct field; no
/// delimiter convention is part of this façade. Convert a selector with
/// [`PortableWorkspaceSubject::try_from`]. Successful conversion validates
/// every identity against the Pale Workspace grammar.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum PortableWorkspaceSubjectSelector {
    /// Select one retained logical source document.
    Source {
        /// Receipt-local source identity.
        source_id: String,
    },
    /// Select one admitted semantic source component.
    Component {
        /// Component identity.
        component_id: String,
    },
    /// Select one module in the admitted release closure.
    Module {
        /// Module identity.
        module_id: String,
    },
    /// Select one canonically owned semantic declaration.
    Declaration {
        /// Canonical owner module.
        owner_module_id: String,
        /// Namespace containing the declaration.
        namespace_id: String,
        /// Declaration identity local to the namespace.
        declaration_id: String,
    },
    /// Select one resolved import path.
    Import {
        /// Module declaring the import.
        importer_module_id: String,
        /// Selected upstream module.
        upstream_module_id: String,
        /// Selected upstream release.
        upstream_release_id: String,
        /// Named upstream export.
        export_id: String,
        /// Namespace selected from the upstream export.
        source_namespace_id: String,
        /// Namespace bound in the importing module.
        local_namespace_id: String,
    },
    /// Select one applied extension.
    Extension {
        /// Module that owns the extension.
        owner_module_id: String,
        /// Namespace that owns the extension rule.
        extension_namespace_id: String,
        /// Namespace of the extended declaration.
        target_namespace_id: String,
        /// Identity of the extended declaration.
        target_declaration_id: String,
    },
    /// Select the effective policy of one declaration.
    Policy {
        /// Canonical owner module.
        owner_module_id: String,
        /// Namespace containing the declaration.
        namespace_id: String,
        /// Declaration identity local to the namespace.
        declaration_id: String,
    },
    /// Select the complete responsibility closure.
    ResponsibilityClosure,
    /// Select the complete optional organizational translation closure.
    OrganizationalTranslationClosure,
    /// Select the complete explicit experience-composition closure.
    ExperienceComposition,
    /// Select one logical or release-bound resource.
    Resource {
        /// Module selecting the resource.
        owner_module_id: String,
        /// Workspace-local resource identity.
        resource_id: String,
    },
    /// Select the exact resolution lock.
    ResolutionLock,
    /// Select the exact universe lock.
    UniverseLock,
    /// Select the complete workspace semantic IR.
    Workspace {
        /// Principal-relative workspace identity.
        workspace_id: String,
    },
}

/// One field of a [`PortableWorkspaceSubjectSelector`] is malformed.
///
/// The error identifies the rejected coordinate member without retaining or
/// echoing the untrusted value.
#[derive(Debug, thiserror::Error)]
#[error("portable workspace subject selector has an invalid {field}")]
pub struct InvalidPortableWorkspaceSubjectSelector {
    field: &'static str,
    #[source]
    source: InvalidWorkspaceLanguageId,
}

impl InvalidPortableWorkspaceSubjectSelector {
    /// Return the stable selector-field name that failed validation.
    #[must_use]
    pub const fn field(&self) -> &'static str {
        self.field
    }
}

/// A well-formed full identity for a compiler-proof subject.
///
/// Obtain a subject from compiler results or validate a typed selector with
/// [`Self::try_from`]. Valid syntax does not prove that any compilation selects
/// the subject. Membership and value comparison remain separate queries.
/// Private storage preserves compound identities without joining them into text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortableWorkspaceSubject {
    pub(super) inner: WorkspaceDerivationSubjectV1,
}

impl PortableWorkspaceSubject {
    pub(crate) fn from_internal(subject: &WorkspaceDerivationSubjectV1) -> Self {
        Self {
            inner: subject.clone(),
        }
    }

    /// Borrow the typed key only for another SDK-owned application projection.
    pub(crate) const fn application_subject(&self) -> &WorkspaceDerivationSubjectV1 {
        &self.inner
    }

    /// Project this validated coordinate into its owned closed selector.
    ///
    /// Passing the result back to [`Self::try_from`] reconstructs the same
    /// subject. The round trip preserves every compound identity separately.
    #[must_use]
    pub fn selector(&self) -> PortableWorkspaceSubjectSelector {
        PortableWorkspaceSubjectSelector::from(self)
    }

    /// Return the closed coordinate family.
    #[must_use]
    pub const fn kind(&self) -> PortableWorkspaceSubjectKind {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Source { .. } => PortableWorkspaceSubjectKind::Source,
            WorkspaceDerivationSubjectV1::Component { .. } => {
                PortableWorkspaceSubjectKind::Component
            }
            WorkspaceDerivationSubjectV1::Module { .. } => PortableWorkspaceSubjectKind::Module,
            WorkspaceDerivationSubjectV1::Declaration { .. } => {
                PortableWorkspaceSubjectKind::Declaration
            }
            WorkspaceDerivationSubjectV1::Import { .. } => PortableWorkspaceSubjectKind::Import,
            WorkspaceDerivationSubjectV1::Extension { .. } => {
                PortableWorkspaceSubjectKind::Extension
            }
            WorkspaceDerivationSubjectV1::Policy { .. } => PortableWorkspaceSubjectKind::Policy,
            WorkspaceDerivationSubjectV1::ResponsibilityClosure => {
                PortableWorkspaceSubjectKind::ResponsibilityClosure
            }
            WorkspaceDerivationSubjectV1::OrganizationalTranslationClosure => {
                PortableWorkspaceSubjectKind::OrganizationalTranslationClosure
            }
            WorkspaceDerivationSubjectV1::ExperienceComposition => {
                PortableWorkspaceSubjectKind::ExperienceComposition
            }
            WorkspaceDerivationSubjectV1::Resource { .. } => PortableWorkspaceSubjectKind::Resource,
            WorkspaceDerivationSubjectV1::ResolutionLock => {
                PortableWorkspaceSubjectKind::ResolutionLock
            }
            WorkspaceDerivationSubjectV1::UniverseLock => {
                PortableWorkspaceSubjectKind::UniverseLock
            }
            WorkspaceDerivationSubjectV1::Workspace { .. } => {
                PortableWorkspaceSubjectKind::Workspace
            }
        }
    }

    /// Receipt-local source identity for [`PortableWorkspaceSubjectKind::Source`].
    #[must_use]
    pub fn source_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Source { source_id } => Some(source_id.as_str()),
            _ => None,
        }
    }

    /// Component identity for [`PortableWorkspaceSubjectKind::Component`].
    #[must_use]
    pub fn component_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Component { component_id } => Some(component_id.as_str()),
            _ => None,
        }
    }

    /// Module identity for [`PortableWorkspaceSubjectKind::Module`].
    #[must_use]
    pub fn module_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Module { module_id } => Some(module_id.as_str()),
            _ => None,
        }
    }

    /// Owning module for declaration, extension, policy, or resource subjects.
    #[must_use]
    pub fn owner_module_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Declaration { owner_module, .. }
            | WorkspaceDerivationSubjectV1::Extension { owner_module, .. }
            | WorkspaceDerivationSubjectV1::Policy { owner_module, .. }
            | WorkspaceDerivationSubjectV1::Resource { owner_module, .. } => {
                Some(owner_module.as_str())
            }
            _ => None,
        }
    }

    /// Namespace of a declaration, policy, or extension target.
    #[must_use]
    pub fn declaration_namespace_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Declaration { declaration, .. }
            | WorkspaceDerivationSubjectV1::Policy { declaration, .. } => {
                Some(declaration.namespace.as_str())
            }
            WorkspaceDerivationSubjectV1::Extension { target, .. } => {
                Some(target.namespace.as_str())
            }
            _ => None,
        }
    }

    /// Local declaration identity for a declaration, policy, or extension target.
    #[must_use]
    pub fn declaration_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Declaration { declaration, .. }
            | WorkspaceDerivationSubjectV1::Policy { declaration, .. } => {
                Some(declaration.declaration_id.as_str())
            }
            WorkspaceDerivationSubjectV1::Extension { target, .. } => {
                Some(target.declaration_id.as_str())
            }
            _ => None,
        }
    }

    /// Module that declares an import path.
    #[must_use]
    pub fn importer_module_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Import {
                importer_module, ..
            } => Some(importer_module.as_str()),
            _ => None,
        }
    }

    /// Upstream module selected by an import path.
    #[must_use]
    pub fn upstream_module_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Import {
                upstream_module, ..
            } => Some(upstream_module.as_str()),
            _ => None,
        }
    }

    /// Exact upstream release selected by an import path.
    #[must_use]
    pub fn upstream_release_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Import {
                upstream_release, ..
            } => Some(upstream_release.as_str()),
            _ => None,
        }
    }

    /// Named upstream export selected by an import path.
    #[must_use]
    pub fn export_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Import { export_id, .. } => Some(export_id.as_str()),
            _ => None,
        }
    }

    /// Upstream namespace selected by an import path.
    #[must_use]
    pub fn source_namespace_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Import {
                source_namespace, ..
            } => Some(source_namespace.as_str()),
            _ => None,
        }
    }

    /// Importer-local namespace bound by an import path.
    #[must_use]
    pub fn local_namespace_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Import {
                local_namespace, ..
            } => Some(local_namespace.as_str()),
            _ => None,
        }
    }

    /// Namespace that owns an extension rule.
    #[must_use]
    pub fn extension_namespace_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Extension { namespace, .. } => Some(namespace.as_str()),
            _ => None,
        }
    }

    /// Workspace-local resource identity for a resource subject.
    #[must_use]
    pub fn resource_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Resource { resource_id, .. } => {
                Some(resource_id.as_str())
            }
            _ => None,
        }
    }

    /// Principal-relative workspace identity for the workspace subject.
    #[must_use]
    pub fn workspace_id(&self) -> Option<&str> {
        match &self.inner {
            WorkspaceDerivationSubjectV1::Workspace { workspace_id } => Some(workspace_id.as_str()),
            _ => None,
        }
    }
}

fn parse_selector_id<T>(
    field: &'static str,
    value: String,
) -> Result<T, InvalidPortableWorkspaceSubjectSelector>
where
    T: TryFrom<String, Error = InvalidWorkspaceLanguageId>,
{
    T::try_from(value).map_err(|source| InvalidPortableWorkspaceSubjectSelector { field, source })
}

fn declaration_selector(
    namespace_id: String,
    declaration_id: String,
) -> Result<WorkspaceDeclarationRefV1, InvalidPortableWorkspaceSubjectSelector> {
    Ok(WorkspaceDeclarationRefV1 {
        namespace: parse_selector_id("namespace_id", namespace_id)?,
        declaration_id: parse_selector_id("declaration_id", declaration_id)?,
    })
}

impl TryFrom<PortableWorkspaceSubjectSelector> for PortableWorkspaceSubject {
    type Error = InvalidPortableWorkspaceSubjectSelector;

    fn try_from(selector: PortableWorkspaceSubjectSelector) -> Result<Self, Self::Error> {
        use PortableWorkspaceSubjectSelector as Selector;

        let inner = match selector {
            Selector::Source { source_id } => WorkspaceDerivationSubjectV1::Source {
                source_id: parse_selector_id::<WorkspaceDerivationSourceId>(
                    "source_id",
                    source_id,
                )?,
            },
            Selector::Component { component_id } => WorkspaceDerivationSubjectV1::Component {
                component_id: parse_selector_id::<WorkspaceComponentId>(
                    "component_id",
                    component_id,
                )?,
            },
            Selector::Module { module_id } => WorkspaceDerivationSubjectV1::Module {
                module_id: parse_selector_id::<WorkspaceModuleId>("module_id", module_id)?,
            },
            Selector::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => WorkspaceDerivationSubjectV1::Declaration {
                owner_module: parse_selector_id::<WorkspaceModuleId>(
                    "owner_module_id",
                    owner_module_id,
                )?,
                declaration: declaration_selector(namespace_id, declaration_id)?,
            },
            Selector::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => WorkspaceDerivationSubjectV1::Import {
                importer_module: parse_selector_id::<WorkspaceModuleId>(
                    "importer_module_id",
                    importer_module_id,
                )?,
                upstream_module: parse_selector_id::<WorkspaceModuleId>(
                    "upstream_module_id",
                    upstream_module_id,
                )?,
                upstream_release: parse_selector_id::<WorkspaceReleaseId>(
                    "upstream_release_id",
                    upstream_release_id,
                )?,
                export_id: parse_selector_id::<WorkspaceExportId>("export_id", export_id)?,
                source_namespace: parse_selector_id::<WorkspaceNamespaceId>(
                    "source_namespace_id",
                    source_namespace_id,
                )?,
                local_namespace: parse_selector_id::<WorkspaceNamespaceId>(
                    "local_namespace_id",
                    local_namespace_id,
                )?,
            },
            Selector::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => WorkspaceDerivationSubjectV1::Extension {
                owner_module: parse_selector_id::<WorkspaceModuleId>(
                    "owner_module_id",
                    owner_module_id,
                )?,
                namespace: parse_selector_id::<WorkspaceNamespaceId>(
                    "extension_namespace_id",
                    extension_namespace_id,
                )?,
                target: WorkspaceDeclarationRefV1 {
                    namespace: parse_selector_id::<WorkspaceNamespaceId>(
                        "target_namespace_id",
                        target_namespace_id,
                    )?,
                    declaration_id: parse_selector_id::<WorkspaceDeclarationId>(
                        "target_declaration_id",
                        target_declaration_id,
                    )?,
                },
            },
            Selector::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => WorkspaceDerivationSubjectV1::Policy {
                owner_module: parse_selector_id::<WorkspaceModuleId>(
                    "owner_module_id",
                    owner_module_id,
                )?,
                declaration: declaration_selector(namespace_id, declaration_id)?,
            },
            Selector::ResponsibilityClosure => WorkspaceDerivationSubjectV1::ResponsibilityClosure,
            Selector::OrganizationalTranslationClosure => {
                WorkspaceDerivationSubjectV1::OrganizationalTranslationClosure
            }
            Selector::ExperienceComposition => WorkspaceDerivationSubjectV1::ExperienceComposition,
            Selector::Resource {
                owner_module_id,
                resource_id,
            } => WorkspaceDerivationSubjectV1::Resource {
                owner_module: parse_selector_id::<WorkspaceModuleId>(
                    "owner_module_id",
                    owner_module_id,
                )?,
                resource_id: parse_selector_id::<WorkspaceResourceId>("resource_id", resource_id)?,
            },
            Selector::ResolutionLock => WorkspaceDerivationSubjectV1::ResolutionLock,
            Selector::UniverseLock => WorkspaceDerivationSubjectV1::UniverseLock,
            Selector::Workspace { workspace_id } => WorkspaceDerivationSubjectV1::Workspace {
                workspace_id: parse_selector_id::<WorkspaceId>("workspace_id", workspace_id)?,
            },
        };
        Ok(Self { inner })
    }
}

impl From<&PortableWorkspaceSubject> for PortableWorkspaceSubjectSelector {
    fn from(subject: &PortableWorkspaceSubject) -> Self {
        use WorkspaceDerivationSubjectV1 as Subject;

        match &subject.inner {
            Subject::Source { source_id } => Self::Source {
                source_id: source_id.to_string(),
            },
            Subject::Component { component_id } => Self::Component {
                component_id: component_id.to_string(),
            },
            Subject::Module { module_id } => Self::Module {
                module_id: module_id.to_string(),
            },
            Subject::Declaration {
                owner_module,
                declaration,
            } => Self::Declaration {
                owner_module_id: owner_module.to_string(),
                namespace_id: declaration.namespace.to_string(),
                declaration_id: declaration.declaration_id.to_string(),
            },
            Subject::Import {
                importer_module,
                upstream_module,
                upstream_release,
                export_id,
                source_namespace,
                local_namespace,
            } => Self::Import {
                importer_module_id: importer_module.to_string(),
                upstream_module_id: upstream_module.to_string(),
                upstream_release_id: upstream_release.to_string(),
                export_id: export_id.to_string(),
                source_namespace_id: source_namespace.to_string(),
                local_namespace_id: local_namespace.to_string(),
            },
            Subject::Extension {
                owner_module,
                namespace,
                target,
            } => Self::Extension {
                owner_module_id: owner_module.to_string(),
                extension_namespace_id: namespace.to_string(),
                target_namespace_id: target.namespace.to_string(),
                target_declaration_id: target.declaration_id.to_string(),
            },
            Subject::Policy {
                owner_module,
                declaration,
            } => Self::Policy {
                owner_module_id: owner_module.to_string(),
                namespace_id: declaration.namespace.to_string(),
                declaration_id: declaration.declaration_id.to_string(),
            },
            Subject::ResponsibilityClosure => Self::ResponsibilityClosure,
            Subject::OrganizationalTranslationClosure => Self::OrganizationalTranslationClosure,
            Subject::ExperienceComposition => Self::ExperienceComposition,
            Subject::Resource {
                owner_module,
                resource_id,
            } => Self::Resource {
                owner_module_id: owner_module.to_string(),
                resource_id: resource_id.to_string(),
            },
            Subject::ResolutionLock => Self::ResolutionLock,
            Subject::UniverseLock => Self::UniverseLock,
            Subject::Workspace { workspace_id } => Self::Workspace {
                workspace_id: workspace_id.to_string(),
            },
        }
    }
}

/// Façade-owned provenance carried by one verified causal fact.
///
/// These variants identify the semantic owner used by compilation. They do not
/// assert native authority, provider observation, runtime activation, or an
/// effect capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortableWorkspaceProvenance {
    /// Fact admitted directly from one retained logical source.
    Source {
        /// Receipt-local source identity.
        source_id: String,
    },
    /// Fact owned by one exact selected module release.
    ModuleRelease {
        /// Owning module.
        module_id: String,
        /// Selected release.
        release_id: String,
        /// Exact sealed module identity.
        module_digest: String,
    },
    /// Fact owned by one resolved import path.
    Import {
        /// Importing module.
        importer_module_id: String,
        /// Selected upstream module.
        upstream_module_id: String,
        /// Selected upstream release.
        upstream_release_id: String,
        /// Named upstream export.
        export_id: String,
        /// Namespace selected upstream.
        source_namespace_id: String,
        /// Namespace bound locally.
        local_namespace_id: String,
    },
    /// Fact owned by one exact extension declaration.
    Extension {
        /// Module that owns the extension.
        owner_module_id: String,
        /// Namespace that owns the extension rule.
        extension_namespace_id: String,
        /// Namespace of the target declaration.
        target_namespace_id: String,
        /// Local identity of the target declaration.
        target_declaration_id: String,
    },
    /// Aggregate fact owned by deterministic workspace compilation.
    Compilation,
}

impl PortableWorkspaceProvenance {
    fn from_internal(origin: &WorkspaceDerivationOriginV1) -> Self {
        match origin {
            WorkspaceDerivationOriginV1::Source { source_id } => Self::Source {
                source_id: source_id.to_string(),
            },
            WorkspaceDerivationOriginV1::ModuleRelease {
                module_id,
                release_id,
                module_digest,
            } => Self::ModuleRelease {
                module_id: module_id.to_string(),
                release_id: release_id.to_string(),
                module_digest: module_digest.to_string(),
            },
            WorkspaceDerivationOriginV1::Import {
                importer_module,
                upstream_module,
                upstream_release,
                export_id,
                source_namespace,
                local_namespace,
            } => Self::Import {
                importer_module_id: importer_module.to_string(),
                upstream_module_id: upstream_module.to_string(),
                upstream_release_id: upstream_release.to_string(),
                export_id: export_id.to_string(),
                source_namespace_id: source_namespace.to_string(),
                local_namespace_id: local_namespace.to_string(),
            },
            WorkspaceDerivationOriginV1::Extension {
                owner_module,
                namespace,
                target,
            } => Self::Extension {
                owner_module_id: owner_module.to_string(),
                extension_namespace_id: namespace.to_string(),
                target_namespace_id: target.namespace.to_string(),
                target_declaration_id: target.declaration_id.to_string(),
            },
            WorkspaceDerivationOriginV1::Compilation => Self::Compilation,
        }
    }
}

/// Façade-owned typed fact produced by one derivation rule.
///
/// Variant-specific fields preserve digest-domain distinctions. A digest in
/// one variant must not be substituted for a differently named digest in
/// another, even when both happen to use lowercase SHA-256 wire spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortableWorkspaceFact {
    /// Exact admitted source coordinates.
    SourceContent {
        /// Confined owner-relative source label, never a host path.
        source_path: String,
        /// SHA-256 of the exact admitted bytes.
        source_digest: String,
        /// Exact admitted byte length.
        byte_len: u32,
    },
    /// Canonical semantic component identity.
    Component {
        /// Component digest.
        component_digest: String,
    },
    /// Canonical selected module identity.
    Module {
        /// Module digest.
        module_digest: String,
    },
    /// Canonically resolved declaration value.
    Declaration {
        /// Domain-separated derived-value digest.
        value_digest: String,
    },
    /// Canonically resolved import surface.
    ImportSurface {
        /// Import-surface digest.
        surface_digest: String,
    },
    /// Resolved extension record, not the resulting target declaration.
    Extension {
        /// Domain-separated derived-value digest.
        value_digest: String,
    },
    /// Canonically accumulated effective policy.
    EffectivePolicy {
        /// Domain-separated derived-value digest.
        value_digest: String,
    },
    /// Complete lowered responsibility set.
    ResponsibilityClosure {
        /// Responsibility-closure digest.
        closure_digest: String,
    },
    /// Complete optional organizational translation.
    OrganizationalTranslationClosure {
        /// Organizational-translation closure digest.
        closure_digest: String,
    },
    /// Complete explicit neutral composition declarations.
    ExperienceComposition {
        /// Composition-source digest.
        source_digest: String,
    },
    /// Canonical selected-resource fact.
    Resource {
        /// Domain-separated derived-value digest.
        value_digest: String,
    },
    /// Exact resolution-lock identity.
    ResolutionLock {
        /// Resolution-lock digest.
        resolution_digest: String,
    },
    /// Exact universe-lock identity.
    UniverseLock {
        /// Universe-lock digest.
        universe_digest: String,
    },
    /// Complete path-free workspace semantic identity.
    SemanticIr {
        /// Workspace semantic-IR digest.
        workspace_semantic_ir_digest: String,
    },
}

impl PortableWorkspaceFact {
    fn from_internal(output: &WorkspaceDerivationOutputV1) -> Self {
        match output {
            WorkspaceDerivationOutputV1::SourceContent {
                source_path,
                source_digest,
                byte_len,
            } => Self::SourceContent {
                source_path: source_path.to_string(),
                source_digest: source_digest.to_string(),
                byte_len: *byte_len,
            },
            WorkspaceDerivationOutputV1::Component { component_digest } => Self::Component {
                component_digest: component_digest.to_string(),
            },
            WorkspaceDerivationOutputV1::Module { module_digest } => Self::Module {
                module_digest: module_digest.to_string(),
            },
            WorkspaceDerivationOutputV1::Declaration { value_digest } => Self::Declaration {
                value_digest: value_digest.to_string(),
            },
            WorkspaceDerivationOutputV1::ImportSurface { surface_digest } => Self::ImportSurface {
                surface_digest: surface_digest.to_string(),
            },
            WorkspaceDerivationOutputV1::Extension { value_digest } => Self::Extension {
                value_digest: value_digest.to_string(),
            },
            WorkspaceDerivationOutputV1::EffectivePolicy { value_digest } => {
                Self::EffectivePolicy {
                    value_digest: value_digest.to_string(),
                }
            }
            WorkspaceDerivationOutputV1::ResponsibilityClosure { closure_digest } => {
                Self::ResponsibilityClosure {
                    closure_digest: closure_digest.to_string(),
                }
            }
            WorkspaceDerivationOutputV1::OrganizationalTranslationClosure { closure_digest } => {
                Self::OrganizationalTranslationClosure {
                    closure_digest: closure_digest.to_string(),
                }
            }
            WorkspaceDerivationOutputV1::ExperienceComposition { source_digest } => {
                Self::ExperienceComposition {
                    source_digest: source_digest.to_string(),
                }
            }
            WorkspaceDerivationOutputV1::Resource { value_digest } => Self::Resource {
                value_digest: value_digest.to_string(),
            },
            WorkspaceDerivationOutputV1::ResolutionLock { resolution_digest } => {
                Self::ResolutionLock {
                    resolution_digest: resolution_digest.to_string(),
                }
            }
            WorkspaceDerivationOutputV1::UniverseLock { universe_digest } => Self::UniverseLock {
                universe_digest: universe_digest.to_string(),
            },
            WorkspaceDerivationOutputV1::SemanticIr {
                workspace_semantic_ir_digest,
            } => Self::SemanticIr {
                workspace_semantic_ir_digest: workspace_semantic_ir_digest.to_string(),
            },
        }
    }
}

/// Application-established equality of a shared subject's typed outputs.
///
/// Source equality includes logical label, byte digest and length. Other facts
/// compare their own digest domains. This does not compare proof ancestry,
/// dependency impact, machine realization or native state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableWorkspaceValueComparison {
    /// Both compiler-owned outputs are equal.
    Equal,
    /// The compiler-owned outputs differ.
    Changed,
}

/// Presence and values of one subject across an admitted compilation pair.
///
/// Only [`Self::Shared`] carries a value comparison. A missing side has no value
/// to compare and is not silently classified as equal or changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortableWorkspaceSubjectValues {
    /// Neither compilation contains the subject.
    Absent,
    /// Only the predecessor contains the subject.
    PriorOnly {
        /// Predecessor's typed output.
        prior: PortableWorkspaceFact,
    },
    /// Only the current compilation contains the subject.
    CurrentOnly {
        /// Current typed output.
        current: PortableWorkspaceFact,
    },
    /// Both compilations contain the same full subject identity.
    Shared {
        /// Predecessor's typed output.
        prior: PortableWorkspaceFact,
        /// Current typed output.
        current: PortableWorkspaceFact,
        /// Equality decided by the application, not by a presentation client.
        comparison: PortableWorkspaceValueComparison,
    },
}

/// Owned answer for one subject in an existing portable comparison.
///
/// The receipt identifies the parent comparison; this value is not a new
/// receipt or an admitted compilation handle. It contains selected typed facts,
/// not source bodies, graph storage or renderer state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableWorkspaceSubjectComparison {
    receipt_digest: String,
    subject: PortableWorkspaceSubject,
    values: PortableWorkspaceSubjectValues,
    canonical_bytes: Vec<u8>,
}

impl PortableWorkspaceSubjectComparison {
    fn new(
        receipt_digest: String,
        subject: PortableWorkspaceSubject,
        values: PortableWorkspaceSubjectValues,
    ) -> Self {
        let mut result = Self {
            receipt_digest,
            subject,
            values,
            canonical_bytes: Vec::new(),
        };
        result.canonical_bytes = super::projection::subject_comparison(&result);
        result
    }

    /// Identity of the existing comparison that produced this answer.
    #[must_use]
    pub fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }

    /// Full semantic subject supplied to the query.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Presence, selected outputs and, only when shared, value comparison.
    #[must_use]
    pub const fn values(&self) -> &PortableWorkspaceSubjectValues {
        &self.values
    }

    /// Canonical binding projection constructed once from this typed answer.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

/// Closed transition kind of one exact source-change root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableWorkspaceChangeKind {
    /// The source subject has no predecessor fact.
    Added,
    /// The source subject exists in both frontiers with different source output.
    Modified,
    /// The source subject exists only in the predecessor frontier.
    Removed,
}

/// Owned projection of one exact source-change root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableWorkspaceChangeRoot {
    root_digest: String,
    subject: PortableWorkspaceSubject,
    change: PortableWorkspaceChange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PortableWorkspaceChange {
    Added {
        current: PortableWorkspaceFact,
    },
    Modified {
        prior: PortableWorkspaceFact,
        current: PortableWorkspaceFact,
    },
    Removed {
        prior: PortableWorkspaceFact,
    },
}

impl PortableWorkspaceChangeRoot {
    pub(super) fn from_internal(
        root: &WorkspaceDerivationRootV1,
        current_nodes: &[WorkspaceDerivationNodeV1],
    ) -> Self {
        let change = match &root.change {
            WorkspaceDerivationRootChangeV1::Added { current_node } => {
                PortableWorkspaceChange::Added {
                    current: Self::current_fact_from_node(current_node, current_nodes),
                }
            }
            WorkspaceDerivationRootChangeV1::Modified {
                prior_output,
                current_node,
            } => PortableWorkspaceChange::Modified {
                prior: PortableWorkspaceFact::from_internal(prior_output),
                current: Self::current_fact_from_node(current_node, current_nodes),
            },
            WorkspaceDerivationRootChangeV1::Removed { prior_output } => {
                PortableWorkspaceChange::Removed {
                    prior: PortableWorkspaceFact::from_internal(prior_output),
                }
            }
        };
        Self {
            root_digest: root.root_digest.to_string(),
            subject: PortableWorkspaceSubject::from_internal(&root.subject),
            change,
        }
    }

    fn current_fact_from_node(
        digest: &pale_core::records::WorkspaceDerivationNodeDigest,
        current_nodes: &[WorkspaceDerivationNodeV1],
    ) -> PortableWorkspaceFact {
        PortableWorkspaceFact::from_internal(
            &current_nodes
                .binary_search_by(|node| node.node_digest.cmp(digest))
                .ok()
                .and_then(|index| current_nodes.get(index))
                .expect("verified change root references one current receipt node")
                .output,
        )
    }

    /// Content address of the exact typed change root.
    #[must_use]
    pub fn root_digest(&self) -> &str {
        &self.root_digest
    }

    /// Semantic source fact that changed.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Closed predecessor/current transition.
    #[must_use]
    pub const fn kind(&self) -> PortableWorkspaceChangeKind {
        match &self.change {
            PortableWorkspaceChange::Added { .. } => PortableWorkspaceChangeKind::Added,
            PortableWorkspaceChange::Modified { .. } => PortableWorkspaceChangeKind::Modified,
            PortableWorkspaceChange::Removed { .. } => PortableWorkspaceChangeKind::Removed,
        }
    }

    /// Exact predecessor fact, absent only for an [`PortableWorkspaceChangeKind::Added`] root.
    #[must_use]
    pub const fn prior_fact(&self) -> Option<&PortableWorkspaceFact> {
        match &self.change {
            PortableWorkspaceChange::Added { .. } => None,
            PortableWorkspaceChange::Modified { prior, .. }
            | PortableWorkspaceChange::Removed { prior } => Some(prior),
        }
    }

    /// Exact current fact, absent only for a [`PortableWorkspaceChangeKind::Removed`] root.
    #[must_use]
    pub const fn current_fact(&self) -> Option<&PortableWorkspaceFact> {
        match &self.change {
            PortableWorkspaceChange::Added { current }
            | PortableWorkspaceChange::Modified { current, .. } => Some(current),
            PortableWorkspaceChange::Removed { .. } => None,
        }
    }
}

/// One bounded page of canonically ordered change roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableWorkspaceChangeRootPage {
    entries: Vec<PortableWorkspaceChangeRoot>,
    next_offset: Option<u32>,
    canonical_bytes: Vec<u8>,
}

impl serde::Serialize for PortableWorkspaceChangeRootPage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(
            &super::projection::ChangeRootPageProjection::from(self),
            serializer,
        )
    }
}

#[cfg(feature = "wire-schema")]
impl schemars::JsonSchema for PortableWorkspaceChangeRootPage {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PortableWorkspaceChangeRootPage".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::PortableWorkspaceChangeRootPage").into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <super::projection::ChangeRootPageProjection<'static> as schemars::JsonSchema>::json_schema(
            generator,
        )
    }
}

impl PortableWorkspaceChangeRootPage {
    fn new(entries: Vec<PortableWorkspaceChangeRoot>, next_offset: Option<u32>) -> Self {
        let mut page = Self {
            entries,
            next_offset,
            canonical_bytes: Vec::new(),
        };
        page.canonical_bytes = super::projection::change_root_page(&page);
        page
    }

    /// Owned root projections in canonical receipt order.
    #[must_use]
    pub fn entries(&self) -> &[PortableWorkspaceChangeRoot] {
        &self.entries
    }

    /// Offset for the next page, or [`None`] when this page is terminal.
    #[must_use]
    pub const fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }

    /// Borrow the API-owned canonical binding projection for this exact page.
    ///
    /// Construction seals the representation once from the same typed entries.
    /// Protocol leaves may decode these bytes, but must not reconstruct the page
    /// shape from the accessors or repeatedly canonicalize it at delivery time.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

/// Closed reachability state for one current causal subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableWorkspaceImpactState {
    /// Reachable from one or more exact change roots.
    Affected,
    /// Outside every exact change-root closure.
    Unaffected,
}

/// Owned projection of one member of the complete current impact partition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableWorkspaceImpact {
    subject: PortableWorkspaceSubject,
    classification: PortableWorkspaceImpactClassification,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PortableWorkspaceImpactClassification {
    Affected {
        contributing_root_count: std::num::NonZeroU32,
    },
    Unaffected,
}

impl PortableWorkspaceImpact {
    fn from_internal(impact: &WorkspaceDerivationImpactV1) -> Self {
        let classification = match &impact.state {
            WorkspaceDerivationImpactStateV1::Affected { root_digests } => {
                PortableWorkspaceImpactClassification::Affected {
                    contributing_root_count: std::num::NonZeroU32::new(
                        u32::try_from(root_digests.len())
                            .expect("verified derivation-root bounds fit u32"),
                    )
                    .expect("verified affected impact carries at least one root"),
                }
            }
            WorkspaceDerivationImpactStateV1::Unaffected => {
                PortableWorkspaceImpactClassification::Unaffected
            }
        };
        Self {
            subject: PortableWorkspaceSubject::from_internal(&impact.subject),
            classification,
        }
    }

    /// Current subject classified by this partition entry.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Root-backed reachability state.
    #[must_use]
    pub const fn state(&self) -> PortableWorkspaceImpactState {
        match self.classification {
            PortableWorkspaceImpactClassification::Affected { .. } => {
                PortableWorkspaceImpactState::Affected
            }
            PortableWorkspaceImpactClassification::Unaffected => {
                PortableWorkspaceImpactState::Unaffected
            }
        }
    }

    /// Exact number of roots whose closure reaches this subject.
    #[must_use]
    pub const fn contributing_root_count(&self) -> u32 {
        match self.classification {
            PortableWorkspaceImpactClassification::Affected {
                contributing_root_count,
            } => contributing_root_count.get(),
            PortableWorkspaceImpactClassification::Unaffected => 0,
        }
    }
}

/// One bounded page of the complete current impact partition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableWorkspaceImpactPage {
    entries: Vec<PortableWorkspaceImpact>,
    next_offset: Option<u32>,
    canonical_bytes: Vec<u8>,
}

impl serde::Serialize for PortableWorkspaceImpactPage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(
            &super::projection::ImpactPageProjection::from(self),
            serializer,
        )
    }
}

#[cfg(feature = "wire-schema")]
impl schemars::JsonSchema for PortableWorkspaceImpactPage {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PortableWorkspaceImpactPage".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::PortableWorkspaceImpactPage").into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <super::projection::ImpactPageProjection as schemars::JsonSchema>::json_schema(generator)
    }
}

impl PortableWorkspaceImpactPage {
    fn new(entries: Vec<PortableWorkspaceImpact>, next_offset: Option<u32>) -> Self {
        let mut page = Self {
            entries,
            next_offset,
            canonical_bytes: Vec::new(),
        };
        page.canonical_bytes = super::projection::impact_page(&page);
        page
    }

    /// Owned impact projections in canonical subject order.
    #[must_use]
    pub fn entries(&self) -> &[PortableWorkspaceImpact] {
        &self.entries
    }

    /// Offset for the next page, or [`None`] when this page is terminal.
    #[must_use]
    pub const fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }

    /// Borrow the API-owned canonical binding projection for this exact page.
    ///
    /// Construction seals these bytes once; repeated transport delivery does
    /// not traverse or canonicalize the impact partition again.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

/// Closed deterministic rule carried by one causal proof step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableWorkspaceDerivationRule {
    /// Admit exact source bytes.
    SourceAdmission,
    /// Admit one semantic source component.
    ComponentAdmission,
    /// Seal one selected module.
    ModuleSealing,
    /// Link one declaration to its canonical owner.
    DeclarationLinking,
    /// Link one exact import surface.
    ImportLinking,
    /// Apply one extension or narrowing.
    ExtensionApplication,
    /// Meet ceilings and accumulate declaration obligations.
    PolicyComposition,
    /// Lower the responsibility-source inventory.
    ResponsibilityLowering,
    /// Lower optional organizational translation.
    OrganizationalTranslation,
    /// Admit explicit neutral experience composition.
    ExperienceComposition,
    /// Retain one logical or release-bound resource selection.
    ResourceSelection,
    /// Bind the resolution lock.
    ResolutionLocking,
    /// Bind the universe lock.
    UniverseLocking,
    /// Seal the complete workspace semantic IR.
    SemanticIrSealing,
}

impl PortableWorkspaceDerivationRule {
    fn from_internal(rule: WorkspaceDerivationRuleV1) -> Self {
        match rule {
            WorkspaceDerivationRuleV1::SourceAdmission => Self::SourceAdmission,
            WorkspaceDerivationRuleV1::ComponentAdmission => Self::ComponentAdmission,
            WorkspaceDerivationRuleV1::ModuleSealing => Self::ModuleSealing,
            WorkspaceDerivationRuleV1::DeclarationLinking => Self::DeclarationLinking,
            WorkspaceDerivationRuleV1::ImportLinking => Self::ImportLinking,
            WorkspaceDerivationRuleV1::ExtensionApplication => Self::ExtensionApplication,
            WorkspaceDerivationRuleV1::PolicyComposition => Self::PolicyComposition,
            WorkspaceDerivationRuleV1::ResponsibilityLowering => Self::ResponsibilityLowering,
            WorkspaceDerivationRuleV1::OrganizationalTranslation => Self::OrganizationalTranslation,
            WorkspaceDerivationRuleV1::ExperienceComposition => Self::ExperienceComposition,
            WorkspaceDerivationRuleV1::ResourceSelection => Self::ResourceSelection,
            WorkspaceDerivationRuleV1::ResolutionLocking => Self::ResolutionLocking,
            WorkspaceDerivationRuleV1::UniverseLocking => Self::UniverseLocking,
            WorkspaceDerivationRuleV1::SemanticIrSealing => Self::SemanticIrSealing,
        }
    }
}

/// One content-addressed fact in a compiler-proof projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableWorkspaceCausalStep {
    node_digest: String,
    subject: PortableWorkspaceSubject,
    provenance: PortableWorkspaceProvenance,
    rule: PortableWorkspaceDerivationRule,
    fact: PortableWorkspaceFact,
}

impl PortableWorkspaceCausalStep {
    pub(super) fn from_internal(node: &WorkspaceDerivationNodeV1) -> Self {
        Self {
            node_digest: node.node_digest.to_string(),
            subject: PortableWorkspaceSubject::from_internal(&node.subject),
            provenance: PortableWorkspaceProvenance::from_internal(&node.origin),
            rule: PortableWorkspaceDerivationRule::from_internal(node.rule),
            fact: PortableWorkspaceFact::from_internal(&node.output),
        }
    }

    /// Content address of this exact proof fact.
    #[must_use]
    pub fn node_digest(&self) -> &str {
        &self.node_digest
    }

    /// Semantic subject produced at this step.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Semantic owner/provenance classification used for this fact.
    #[must_use]
    pub const fn provenance(&self) -> &PortableWorkspaceProvenance {
        &self.provenance
    }

    /// Deterministic compiler rule applied at this step.
    #[must_use]
    pub const fn rule(&self) -> PortableWorkspaceDerivationRule {
        self.rule
    }

    /// Exact typed fact produced by [`Self::rule`].
    #[must_use]
    pub const fn fact(&self) -> &PortableWorkspaceFact {
        &self.fact
    }
}

/// Which admitted proof DAG owns a causal witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableWorkspaceExplanationFrontier {
    /// Predecessor DAG, used only for a removed root.
    Predecessor,
    /// Current DAG, used for an added or modified root.
    Current,
}

/// One deterministic causal witness for an affected current subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableWorkspaceCausalExplanation {
    root: PortableWorkspaceChangeRoot,
    frontier: PortableWorkspaceExplanationFrontier,
    steps: Vec<PortableWorkspaceCausalStep>,
}

impl PortableWorkspaceCausalExplanation {
    /// Deterministically selected exact change root.
    #[must_use]
    pub const fn root(&self) -> &PortableWorkspaceChangeRoot {
        &self.root
    }

    /// Admitted proof DAG traversed by [`Self::steps`].
    #[must_use]
    pub const fn frontier(&self) -> PortableWorkspaceExplanationFrontier {
        self.frontier
    }

    /// Shortest root-to-target fact sequence, including both endpoints.
    #[must_use]
    pub fn steps(&self) -> &[PortableWorkspaceCausalStep] {
        &self.steps
    }
}

/// Closed result state of one causal explanation query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableWorkspaceExplanationState {
    /// Subject is absent from this comparison's current partition.
    NotPresent,
    /// Subject is present and deliberately outside every root closure.
    Unaffected,
    /// Subject is present and has one projected causal witness.
    Affected,
}

/// Owned result of one causal explanation query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableWorkspaceExplanation {
    subject: PortableWorkspaceSubject,
    outcome: PortableWorkspaceExplanationOutcome,
    canonical_bytes: Vec<u8>,
}

impl serde::Serialize for PortableWorkspaceExplanation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(
            &super::projection::ExplanationProjection::from(self),
            serializer,
        )
    }
}

#[cfg(feature = "wire-schema")]
impl schemars::JsonSchema for PortableWorkspaceExplanation {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PortableWorkspaceExplanation".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::PortableWorkspaceExplanation").into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <super::projection::ExplanationProjection<'static> as schemars::JsonSchema>::json_schema(
            generator,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PortableWorkspaceExplanationOutcome {
    NotPresent,
    Unaffected,
    Affected {
        causal: Box<PortableWorkspaceCausalExplanation>,
    },
}

impl PortableWorkspaceExplanation {
    fn not_present(subject: PortableWorkspaceSubject) -> Self {
        Self::new(subject, PortableWorkspaceExplanationOutcome::NotPresent)
    }

    fn unaffected(subject: PortableWorkspaceSubject) -> Self {
        Self::new(subject, PortableWorkspaceExplanationOutcome::Unaffected)
    }

    fn affected(
        subject: PortableWorkspaceSubject,
        causal: PortableWorkspaceCausalExplanation,
    ) -> Self {
        Self::new(
            subject,
            PortableWorkspaceExplanationOutcome::Affected {
                causal: Box::new(causal),
            },
        )
    }

    fn new(
        subject: PortableWorkspaceSubject,
        outcome: PortableWorkspaceExplanationOutcome,
    ) -> Self {
        let mut explanation = Self {
            subject,
            outcome,
            canonical_bytes: Vec::new(),
        };
        explanation.canonical_bytes = super::projection::explanation(&explanation);
        explanation
    }

    /// Exact query outcome.
    #[must_use]
    pub const fn state(&self) -> PortableWorkspaceExplanationState {
        match &self.outcome {
            PortableWorkspaceExplanationOutcome::NotPresent => {
                PortableWorkspaceExplanationState::NotPresent
            }
            PortableWorkspaceExplanationOutcome::Unaffected => {
                PortableWorkspaceExplanationState::Unaffected
            }
            PortableWorkspaceExplanationOutcome::Affected { .. } => {
                PortableWorkspaceExplanationState::Affected
            }
        }
    }

    /// Subject supplied to the query.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Causal witness when [`PortableWorkspaceExplanationState::Affected`].
    #[must_use]
    pub fn causal(&self) -> Option<&PortableWorkspaceCausalExplanation> {
        match &self.outcome {
            PortableWorkspaceExplanationOutcome::Affected { causal } => Some(causal.as_ref()),
            PortableWorkspaceExplanationOutcome::NotPresent
            | PortableWorkspaceExplanationOutcome::Unaffected => None,
        }
    }

    /// Borrow the API-owned canonical binding projection of this explanation.
    ///
    /// The projection is sealed once during construction and retains the
    /// complete typed causal witness when present. It neither recomputes
    /// reachability nor introduces another proof owner.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

struct PageWindow {
    start: usize,
    end: usize,
    next_offset: Option<u32>,
}

impl PageWindow {
    fn new(total: u32, offset: u32, limit: u16) -> Self {
        let limit = if limit == 0 {
            DEFAULT_PROOF_PAGE_SIZE
        } else {
            limit.min(MAX_PROOF_PAGE_SIZE)
        };
        let start = offset.min(total);
        let end = start.saturating_add(u32::from(limit)).min(total);
        Self {
            start: usize::try_from(start)
                .expect("receipt length was converted from the platform usize domain"),
            end: usize::try_from(end)
                .expect("receipt length was converted from the platform usize domain"),
            next_offset: (end < total).then_some(end),
        }
    }

    const fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

#[cfg(test)]
#[path = "../../tests/internal/portable_workspace/derivation.rs"]
mod tests;
