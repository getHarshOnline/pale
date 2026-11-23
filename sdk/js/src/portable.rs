//! Portable and effect-free PaleJS projections.
//!
//! The shallow projection is organized by contract, diagnostic, workspace,
//! derivation, inspection, native-action, and [`Pale`] façade ownership. Concept
//! fragments share this one private module so napi-rs receives one stable ABI
//! surface without an internal re-export graph.

use napi::bindgen_prelude::{Buffer, Either, Undefined};
use napi::{Env, Result};
use napi_derive::napi;
use pale_sdk::{
    ContractDocument, DiagnosticDisposition as ApiDiagnosticDisposition,
    DiagnosticEffectCertainty as ApiDiagnosticEffectCertainty,
    DiagnosticProjection as ApiDiagnosticProjection, DiagnosticRetry as ApiDiagnosticRetry,
    DiagnosticStage as ApiDiagnosticStage, DiagnosticV1,
    NativeActionDescriptor as ApiNativeActionDescriptor, NativeActionPayloadRequest,
    NativeActionRequest, PortableWorkspaceCompilation as ApiPortableWorkspaceCompilation,
    PortableWorkspaceDerivationRule, PortableWorkspaceDocument as ApiPortableWorkspaceDocument,
    PortableWorkspaceExplanation, PortableWorkspaceExplanationFrontier,
    PortableWorkspaceExplanationState, PortableWorkspaceFact, PortableWorkspaceImpact,
    PortableWorkspaceImpactState, PortableWorkspaceProvenance,
    PortableWorkspaceRequest as ApiPortableWorkspaceRequest, PortableWorkspaceSubject,
    PortableWorkspaceSubjectKind,
};

use crate::authoring::{authoring_page_limit, AuthoringOccurrence, AuthoringSourceLocation};
use crate::error::diagnostic_error;
use crate::Pale;

include!("portable/contract.rs");
include!("portable/diagnostic.rs");
include!("portable/workspace.rs");
include!("portable/native_action.rs");
include!("portable/derivation.rs");
include!("portable/inspection.rs");
include!("portable/impact_roots.rs");
include!("portable/proof_dependents.rs");
include!("portable/proof_dependencies.rs");
include!("portable/comparison_subjects.rs");
include!("portable/facade.rs");

include!("portable/organization.rs");
include!("portable/organization_values.rs");
include!("portable/organization_correspondence.rs");
include!("portable/recipient_inspection.rs");
#[cfg(test)]
#[path = "../tests/internal/organization.rs"]
mod organization_tests;
