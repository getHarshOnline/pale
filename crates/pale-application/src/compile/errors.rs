//! Typed failures from input loading through compilation re-verification.
//!
//! [`CompileError`] preserves failures owned by input layers, while
//! [`CompilationInvariantError`] names contradictions in compiler-produced
//! state. Callers can therefore distinguish invalid source from an application bug.

use crate::{packages, plan, semantic, workspace};

/// Typed failure to construct or re-verify one coherent compiler result.
#[derive(Debug, thiserror::Error)]
pub enum CompilationInvariantError {
    /// Profile/instance identity verification failed with its original source.
    #[error("compiled project identity is invalid: {source}")]
    ProjectIdentity {
        /// Exact underlying identity failure.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Selected Package content/version identities cannot be structurally joined.
    #[error("selected package version/content identities do not have the same keys")]
    PackageBindingKeyMismatch,
    /// A semantic IR invariant failed with its original typed source.
    #[error("semantic IR invariant failed: {0}")]
    SemanticIr(#[from] semantic::SemanticError),
    /// Selection-bounded Package graph verification failed.
    #[error("Package semantic IR invariant failed: {0}")]
    PackageGraph(#[from] packages::PackageError),
    /// Neutral composition compilation failed with its original typed source.
    #[error("experience composition invariant failed: {0}")]
    Composition(#[from] crate::composition::CompositionError),
    /// Canonical identity construction failed with its original source.
    #[error("cannot seal {coordinate}: {source}")]
    Canonicalization {
        /// Coordinate being constructed.
        coordinate: &'static str,
        /// Exact canonicalization failure.
        #[source]
        source: pale_core::canon::CanonError,
    },
    /// Rebuilding a repository manifest failed with its original plan source.
    #[error("cannot rebuild repository `{repository}` manifest: {source}")]
    ManifestRebuild {
        /// Logical repository whose manifest was rebuilt.
        repository: String,
        /// Exact plan-layer failure.
        #[source]
        source: Box<plan::PlanError>,
    },
    /// An exact admitted coordinate changed before or after sealing.
    #[error("compiled coordinate changed: {coordinate}")]
    CoordinateDrift {
        /// Exact coordinate whose recomputation disagreed.
        coordinate: &'static str,
    },
    /// A bounded structural invariant failed without an underlying error source.
    #[error("{coordinate}: {detail}")]
    Structure {
        /// Exact compiler coordinate being checked.
        coordinate: &'static str,
        /// Deterministic bounded invariant detail.
        detail: String,
    },
}

impl CompilationInvariantError {
    /// Construct a bounded contradiction attributed to one compiler coordinate.
    pub(super) fn structure(coordinate: &'static str, detail: impl Into<String>) -> Self {
        Self::Structure {
            coordinate,
            detail: detail.into(),
        }
    }
}

/// Errors from the compile pipeline.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    /// An inspectable execution workspace has no path for producing host surfaces.
    #[error("execution workspace `{execution_workspace}` needs at least one execution_path_ids entry before rendering")]
    MissingExecutionPaths {
        /// Logical execution workspace whose path selection must be completed.
        execution_workspace: pale_core::records::LogicalExecutionWorkspaceIdV1,
    },
    /// Owned logical documents violate a captured-input confinement or resource law.
    #[error(transparent)]
    CapturedInput(Box<crate::captured_input::CapturedCompilationInputError>),
    /// The compiled schema descriptor registry could not establish its exact identity.
    #[error(transparent)]
    SchemaRegistry(Box<pale_contracts::SchemaError>),
    /// The distribution lock was authored for a different descriptor manifest or schema set.
    #[error(
        "distribution lock pins schema manifest `{}` and schema set `{}`, but the running compiler embeds schema manifest `{}` and schema set `{}`",
        pinned.schema_manifest_sha256,
        pinned.schema_set_sha256,
        running.schema_manifest_sha256,
        running.schema_set_sha256
    )]
    SchemaRegistryMismatch {
        /// Exact schema-registry identity pinned by the distribution lock.
        pinned: pale_core::records::SchemaRegistryIdentity,
        /// Exact schema-registry identity embedded in the running compiler.
        running: pale_core::records::SchemaRegistryIdentity,
    },
    /// Supplied composition declarations failed identity or relationship admission.
    #[error(transparent)]
    CompositionInput(Box<crate::composition::CompositionError>),
    /// Compiler-produced records could not form one coherent one-shot result.
    #[error(transparent)]
    CompilationInvariant(Box<CompilationInvariantError>),
    /// Package loading failed before the selected distribution could be composed.
    #[error(transparent)]
    PackageLoad(Box<packages::PackageError>),
    /// The compiled native-action catalog could not establish one closed registry.
    #[error("native action registry: {0}")]
    NativeActionRegistry(#[source] Box<pale_core::records::ActionError>),
    /// Project, repository, capability, machine, or lock input is invalid.
    #[error(transparent)]
    Workspace(Box<workspace::WorkspaceError>),
    /// The rendered repository projection cannot form a valid materialization plan.
    #[error(transparent)]
    Plan(Box<plan::PlanError>),
    /// The lock names a different compiler contract from the running compiler.
    #[error("distribution lock pins compiler `{pinned}`, but the running compiler is `{running}`")]
    CompilerIdentityMismatch {
        /// Compiler contract pinned by the distribution lock.
        pinned: String,
        /// Compiler contract implemented by this binary.
        running: String,
    },
    /// The selected path cannot resolve a registered terminal provider.
    #[error("execution path `{execution_path}` cannot select a terminal provider: {source}")]
    TerminalProvider {
        /// Selected execution path whose provider must be corrected.
        execution_path: pale_core::records::ExecutionPathSelectionIdV1,
        /// Original provider-resolution failure.
        #[source]
        source: crate::terminal_adapters::TerminalAdapterResolutionError,
    },
    /// A selected path needs an event excluded by the session's declared limits.
    #[error("execution path `{execution_path}` requires `{}` in session lifecycle_events", event.as_str())]
    LifecycleConflict {
        /// Selected execution path requiring the omitted event.
        execution_path: pale_core::records::ExecutionPathSelectionIdV1,
        /// Required event omitted by the session declaration.
        event: pale_core::records::LifecycleEvent,
        /// Events the session currently permits.
        declared: Vec<pale_core::records::LifecycleEvent>,
    },
    /// Live machine verification failed after portable inputs were resolved.
    #[error("activation verification: {0}")]
    Verify(#[source] Box<workspace::WorkspaceError>),
    /// A repository projection could not be rendered faithfully.
    #[error(transparent)]
    Render(Box<crate::render::RenderError>),
    /// Selected semantic packages could not form a valid repository graph.
    #[error(transparent)]
    Semantic(Box<semantic::SemanticError>),
}

macro_rules! boxed_compile_error_from {
    ($source:ty, $variant:ident) => {
        impl From<$source> for CompileError {
            fn from(source: $source) -> Self {
                Self::$variant(Box::new(source))
            }
        }
    };
}

boxed_compile_error_from!(pale_contracts::SchemaError, SchemaRegistry);
boxed_compile_error_from!(
    crate::captured_input::CapturedCompilationInputError,
    CapturedInput
);
boxed_compile_error_from!(packages::PackageError, PackageLoad);
boxed_compile_error_from!(workspace::WorkspaceError, Workspace);
boxed_compile_error_from!(plan::PlanError, Plan);
boxed_compile_error_from!(crate::render::RenderError, Render);
boxed_compile_error_from!(semantic::SemanticError, Semantic);

impl From<CompilationInvariantError> for CompileError {
    fn from(source: CompilationInvariantError) -> Self {
        Self::CompilationInvariant(Box::new(source))
    }
}
