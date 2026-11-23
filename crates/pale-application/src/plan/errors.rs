//! Typed failures from effect-free materialization planning.
//!
//! [`PlanError`] distinguishes invalid compiler output from target-observation
//! contradictions and canonical identity failures. Planning never performs the
//! filesystem effect whose intent it describes.

use pale_core::{canon, records::LogicalRepositoryIdV1};

use crate::compile::CompilationInvariantError;
use crate::render::RenderError;

/// Errors from deterministic plan construction.
///
/// Saved-plan self-verification, class checks, and equality admission belong to
/// [`pale_core::admission::admit_plan`], which returns the capability
/// required for mutation.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    /// One-shot compiler output was mutated or recombined after admission.
    #[error("compiled input is invalid: {0}")]
    InvalidCompilation(#[source] Box<CompilationInvariantError>),
    /// Project or repository identity cannot be encoded as a confined plan identity.
    #[error("project identity is invalid: {source}")]
    InvalidIdentity {
        /// Identity-verification failure retained for diagnostic source traversal.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Generated-surface manifest violates its structural or identity invariants.
    #[error("generated manifest is invalid: {0}")]
    InvalidManifest(String),
    /// More than one generated surface claims the same repository target.
    #[error("duplicate generated surface target `{0}`")]
    DuplicateSurfaceTarget(String),
    /// Captured pre-state contains more than one baseline for the same target.
    #[error("duplicate baseline target `{0}`")]
    DuplicateBaselineTarget(String),
    /// Captured pre-state is incomplete or internally inconsistent.
    #[error("invalid target baseline: {0}")]
    InvalidBaseline(String),
    /// A caller-supplied portable plan or machine-local effect set is inconsistent.
    #[error("invalid supplied materialization plan: {0}")]
    InvalidSuppliedPlan(String),
    /// Rendered content contradicts its declared ownership or digest contract.
    #[error("invalid generated surface: {0}")]
    InvalidSurface(#[source] RenderError),
    /// Persisted ownership state is malformed, stale, or inconsistent with the manifest.
    #[error("invalid materialization state: {0}")]
    InvalidMaterializationState(String),
    /// A caller requested a repository identity absent from the compilation.
    #[error("invalid run target `{0}`")]
    InvalidRunTarget(String),
    /// The selected execution-workspace edge admits observation but not repository mutation.
    #[error("repository `{repository}` is read-only in the selected execution workspace")]
    RepositoryMutationDenied {
        /// Canonical repository whose sealed membership carries the read ceiling.
        repository: LogicalRepositoryIdV1,
    },
    /// Per-target results cannot represent one ordered execution-workspace run.
    #[error("invalid execution-workspace run result: {0}")]
    InvalidRunResult(String),
    /// Transaction evidence does not prove the admitted plan's exact effects.
    #[error("invalid materialization receipt: {0}")]
    InvalidReceipt(String),
    /// Observed ownership-lineage evidence does not match the admitted state transition.
    #[error("materialization lineage evidence is `{actual}`, expected `{expected}` for the admitted plan")]
    InvalidLineageTransition {
        /// Lineage transition required by the plan's prior and next ownership state.
        expected: &'static str,
        /// Lineage transition supplied by the transaction executor.
        actual: &'static str,
    },
    /// Existing owned state and the new surface disagree on whole-file versus managed-block ownership.
    #[error("ownership mode changed at `{0}`; remove it before changing ownership")]
    OwnershipTransition(String),
    /// Application-produced planning state contradicts a closed construction law.
    #[error("materialization planning invariant `{coordinate}` failed: {detail}")]
    PlanInvariant {
        /// Stable planning stage that produced the contradiction.
        coordinate: &'static str,
        /// Bounded deterministic explanation of the failed law.
        detail: String,
    },
    /// Canonical encoding or digest computation failed while sealing the plan.
    #[error(transparent)]
    Canon(#[from] canon::CanonError),
    /// The core-owned surface-plan preimage violates its closure or canonical identity law.
    #[error("materialization plan could not be sealed: {0}")]
    PlanSeal(#[from] pale_core::records::AgentSurfacePlanSealErrorV1),
    /// The core-owned receipt preimage violates its closure or canonical identity law.
    #[error("materialization receipt could not be sealed: {0}")]
    ReceiptSeal(#[from] pale_core::records::MaterializationReceiptSealErrorV1),
    /// A generated plan boundary violates its pinned JSON schema.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
}

impl From<CompilationInvariantError> for PlanError {
    fn from(source: CompilationInvariantError) -> Self {
        Self::InvalidCompilation(Box::new(source))
    }
}
