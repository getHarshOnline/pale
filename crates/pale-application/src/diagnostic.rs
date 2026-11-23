//! Build and read the diagnostics that explain a failure, without letting display
//! text decide what failed.
//!
//! `actions` and `resident` build on it; it builds on no other module in this
//! crate. Operation owners select the failure classification. This module redacts
//! and bounds new text, validates the existing diagnostic contract, and derives
//! its content identity. Stored diagnostics are checked without rewriting
//! evidence.

mod admission;
mod failure;
mod projection;
mod schema;
mod text;

pub use admission::{AdmittedDiagnostic, DiagnosticAdmissionError};
pub use failure::{project_failure, CauseProjection, FailureSpecification};
pub use projection::DiagnosticProjection;

#[cfg(test)]
#[path = "../tests/internal/diagnostic/admission.rs"]
mod admission_tests;

#[cfg(test)]
#[path = "../tests/internal/diagnostic/projection.rs"]
mod projection_tests;
pub use schema::{
    schema_diagnostic_coordinate, schema_violation_projection, SchemaViolationProjection,
};
