//! API failures with structured diagnostics and controlled access to their causes.

use std::error::Error;
use std::fmt;

use pale_application::ApplicationFailureDisposition;
use pale_core::records::{DiagnosticDispositionV1, DiagnosticRetryV1, RecordSchemaIdentity};

use crate::{AuthoringCorrection, AuthoringSourceLocation, DiagnosticV1};
use pale_application::diagnostic::CauseProjection;
pub(crate) use pale_application::diagnostic::FailureSpecification;

mod composition;
pub(crate) use composition::composition_failure_details;

/// Project one application-owned consequence into portable diagnostic policy.
pub(crate) const fn application_diagnostic_policy(
    disposition: ApplicationFailureDisposition,
) -> (DiagnosticDispositionV1, DiagnosticRetryV1) {
    match disposition {
        ApplicationFailureDisposition::Operational => (
            DiagnosticDispositionV1::OperationalFailure,
            DiagnosticRetryV1::NotIndicated,
        ),
        ApplicationFailureDisposition::InvalidInput => (
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
        ApplicationFailureDisposition::Blocked => (
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
        ),
        ApplicationFailureDisposition::Internal => (
            DiagnosticDispositionV1::InternalFailure,
            DiagnosticRetryV1::NotIndicated,
        ),
    }
}

/// API construction failed while checking the embedded contracts or computations.
///
/// These checks run before the API can accept calls and have no external effects.
/// [`Self::code`] identifies the failure category. [`std::error::Error::source`]
/// gives Rust callers the underlying typed cause when one exists, without adding
/// internal contract or computation types to the public error fields.
pub struct ApiInitializationError {
    kind: InitializationFailure,
    source: Option<Box<dyn Error + Send + Sync>>,
}

#[derive(Clone, Copy, Debug)]
enum InitializationFailure {
    /// Embedded descriptor, schema, or diagnostic identity is incoherent.
    Contracts,
    /// Active contract cardinality cannot fit the portable counter.
    SchemaCountOverflow,
    /// Compiled pure-action registry contradicts its embedded contracts.
    Actions,
}

impl ApiInitializationError {
    /// Preserve an embedded-contract cause behind the opaque façade error.
    pub(crate) fn contracts(source: pale_contracts::SchemaError) -> Self {
        Self {
            kind: InitializationFailure::Contracts,
            source: Some(Box::new(source)),
        }
    }

    /// Construct the source-free cardinality failure.
    pub(crate) fn schema_count_overflow() -> Self {
        Self {
            kind: InitializationFailure::SchemaCountOverflow,
            source: None,
        }
    }

    /// Preserve a pure-action registry cause behind the opaque façade error.
    pub(crate) fn actions(source: pale_core::records::ActionError) -> Self {
        Self {
            kind: InitializationFailure::Actions,
            source: Some(Box::new(source)),
        }
    }

    /// Stable machine-readable category for initialization failure handling.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self.kind {
            InitializationFailure::Contracts => "contract_registry_incoherent",
            InitializationFailure::SchemaCountOverflow => "schema_count_overflow",
            InitializationFailure::Actions => "native_action_registry_incoherent",
        }
    }
}

impl fmt::Debug for ApiInitializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiInitializationError")
            .field("code", &self.code())
            .field("source", &self.source.as_ref().map(|_| "<preserved>"))
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ApiInitializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            InitializationFailure::Contracts => {
                formatter.write_str("embedded contract registry is incoherent")
            }
            InitializationFailure::SchemaCountOverflow => {
                formatter.write_str("embedded schema member count exceeds the portable API bound")
            }
            InitializationFailure::Actions => {
                formatter.write_str("compiled native action registry is incoherent")
            }
        }
    }
}

impl Error for ApiInitializationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

/// A failed API call with a structured explanation and its underlying cause.
///
/// Read [`Self::diagnostic`] for machine-readable failure and retry information.
/// [`Display`](fmt::Display) shows only the summary, within its length limit.
/// [`std::error::Error::source`] exposes the typed Rust cause only when its error
/// chain is safe to show publicly. Otherwise the cause stays internal and callers
/// receive the structured diagnostic without access to that chain.
pub struct ApiError {
    diagnostic: Box<DiagnosticV1>,
    authoring_corrections: Vec<AuthoringCorrection>,
    authoring_location: Option<AuthoringSourceLocation>,
    source: Box<dyn Error + Send + Sync>,
    cause_projection: CauseProjection,
}

impl ApiError {
    /// Project a typed source and its bounded classification into one API failure.
    pub(crate) fn new<E>(
        identity: &RecordSchemaIdentity,
        source: E,
        specification: FailureSpecification<'_>,
    ) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::project(identity, source, specification, CauseProjection::Redacted)
    }

    /// Preserve a typed source whose rendered chain is not safe diagnostic data.
    pub(crate) fn new_with_opaque_source<E>(
        identity: &RecordSchemaIdentity,
        source: E,
        specification: FailureSpecification<'_>,
    ) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::project(identity, source, specification, CauseProjection::Omitted)
    }

    fn project<E>(
        identity: &RecordSchemaIdentity,
        source: E,
        specification: FailureSpecification<'_>,
        cause_projection: CauseProjection,
    ) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        let diagnostic = pale_application::diagnostic::project_failure(
            identity,
            &source,
            specification,
            cause_projection,
        )
        .expect("API-owned failure projection preserves diagnostic.v1");
        Self::from_admitted(source, diagnostic, cause_projection)
    }

    /// Retain a typed cause beside the application-owned diagnostic.
    pub(crate) fn from_admitted<E>(
        source: E,
        diagnostic: pale_application::diagnostic::AdmittedDiagnostic,
        cause_projection: CauseProjection,
    ) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            diagnostic: Box::new(DiagnosticV1::from_admitted(diagnostic)),
            authoring_corrections: Vec::new(),
            authoring_location: None,
            source: Box::new(source),
            cause_projection,
        }
    }

    /// Borrow the checked, machine-readable explanation of this failure.
    #[must_use]
    pub fn diagnostic(&self) -> &DiagnosticV1 {
        &self.diagnostic
    }

    /// Borrow deterministic edits proved against the input of this failed call.
    #[must_use]
    pub fn authoring_corrections(&self) -> &[AuthoringCorrection] {
        &self.authoring_corrections
    }

    /// Borrow the source location established by the component that failed.
    ///
    /// This location retains the actual field and index path; it is not inferred
    /// from a diagnostic's displayed JSON Pointer or decoder message. A missing
    /// value means the failing component could not establish that exact path.
    #[must_use]
    pub const fn authoring_location(&self) -> Option<&AuthoringSourceLocation> {
        self.authoring_location.as_ref()
    }

    /// Attach one exact owner-retained source coordinate.
    pub(crate) fn with_authoring_location(
        mut self,
        location: Option<AuthoringSourceLocation>,
    ) -> Self {
        self.authoring_location = location;
        self
    }

    /// Attach one compiler-proved edit after the diagnostic identity is sealed.
    pub(crate) fn with_authoring_correction(
        mut self,
        evidence: &pale_application::workspace::AuthoringCorrectionEvidence,
    ) -> Self {
        self.authoring_corrections
            .push(AuthoringCorrection::from_evidence(
                evidence,
                self.diagnostic.identity().clone(),
            ));
        self
    }

    /// Take the checked diagnostic, consuming the error.
    #[must_use]
    pub fn into_diagnostic(self) -> DiagnosticV1 {
        *self.diagnostic
    }
}

impl fmt::Debug for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiError")
            .field("diagnostic", &self.diagnostic)
            .field(
                "authoring_correction_count",
                &self.authoring_corrections.len(),
            )
            .field("source", &"<preserved>")
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.diagnostic.summary())
    }
}

impl Error for ApiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.cause_projection {
            CauseProjection::Redacted => Some(self.source.as_ref()),
            CauseProjection::Omitted => None,
        }
    }
}

/// Generic byte-boundary rejection with no domain payload.
#[derive(Debug, thiserror::Error)]
pub(crate) enum BoundaryError {
    /// Input exceeded its owning boundary before parsing.
    #[error("request exceeds the {limit}-byte API boundary")]
    InputTooLarge { limit: usize },
    /// A payload-only action request did not contain a JSON object.
    #[error("native-action payload must be a JSON object")]
    PayloadNotObject,
    /// A known or unknown schema is absent from the explicit façade selection.
    #[error("requested contract is unavailable from this API")]
    ContractNotExposed,
}

#[cfg(test)]
#[path = "../tests/internal/error.rs"]
mod tests;
