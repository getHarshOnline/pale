//! Typed failures at the semantic-plan-to-surface boundary.
//!
//! [`RenderError`] retains whether input state was contradictory or a specific
//! projection could not be produced. It never recasts rendering failure as a
//! target mutation result.

/// Thread-safe typed source retained across renderer error boundaries.
type BoxedSource = Box<dyn std::error::Error + Send + Sync>;

/// A failure to produce a complete, integrity-bound surface.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// Distinct selected resources resolve to one host name.
    #[error("host {kind} name `{name}` is shared by `{first}` and `{second}`")]
    HostNameCollision {
        /// Host namespace in which the names conflict.
        kind: &'static str,
        /// Generated host name.
        name: String,
        /// First canonical resource identity.
        first: String,
        /// Second canonical resource identity.
        second: String,
    },
    /// Inputs that are individually typed do not form one coherent projection.
    #[error("render input `{field}` is invalid: {reason}")]
    InvalidInput {
        /// Input relationship that failed admission.
        field: &'static str,
        /// Exact failed invariant.
        reason: String,
    },
    /// A typed input contract rejected a render field before projection.
    #[error(
        "render input `{field}` is invalid{record}: {source}",
        record = subject
            .as_ref()
            .map(|value| format!(" for `{value}`"))
            .unwrap_or_default()
    )]
    InvalidInputSource {
        /// Input relationship that failed admission.
        field: &'static str,
        /// Optional record identity whose typed contract was rejected.
        subject: Option<String>,
        /// Typed input-contract failure retained for diagnostic source traversal.
        #[source]
        source: BoxedSource,
    },
    /// A generated record or managed block cannot be encoded faithfully.
    #[error("failed to render {artifact}: {reason}")]
    Projection {
        /// Logical artifact being produced.
        artifact: String,
        /// Exact projection or encoding failure.
        reason: String,
    },
    /// A typed encoder, schema, or canonicalization failure prevented projection.
    #[error("failed to render {artifact}: {source}")]
    ProjectionSource {
        /// Logical artifact being produced.
        artifact: String,
        /// Typed projection failure retained for diagnostic source traversal.
        #[source]
        source: BoxedSource,
    },
}

impl RenderError {
    /// Attribute a deterministic rejection to invalid resolved input.
    pub(in crate::render) fn invalid_input(field: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidInput {
            field,
            reason: reason.into(),
        }
    }

    /// Retain the typed source that rejected one resolved input field.
    pub(in crate::render) fn invalid_input_error(
        field: &'static str,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::InvalidInputSource {
            field,
            subject: None,
            source: Box::new(source),
        }
    }

    /// Retain a typed input failure together with the rejected record identity.
    pub(in crate::render) fn invalid_input_record_error(
        field: &'static str,
        subject: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::InvalidInputSource {
            field,
            subject: Some(subject.into()),
            source: Box::new(source),
        }
    }

    /// Attribute a deterministic rejection to projection or serialization.
    pub(in crate::render) fn projection(
        artifact: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::Projection {
            artifact: artifact.into(),
            reason: reason.into(),
        }
    }

    /// Retain the typed source that prevented one artifact from being projected.
    pub(in crate::render) fn projection_error(
        artifact: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ProjectionSource {
            artifact: artifact.into(),
            source: Box::new(source),
        }
    }
}
