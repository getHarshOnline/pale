//! Compile supplied Pale Workspace documents from JavaScript or TypeScript.
//!
//! [`PaleWasm`] exposes the portable, effect-free operations of [`pale_sdk`].
//! Rust owns validation and compilation. Callers supply strings and source
//! bytes, then use handles to inspect the validated results. Result bytes are
//! copied into JavaScript only when requested.
//!
//! This SDK cannot read files, start processes, contact the resident or
//! providers, or install, activate, or apply changes. A successful compilation
//! does not prove that a machine or provider is ready.

use pale_sdk::{
    capability_inventory, ApiError, ApiLeaf, AuthoringCorrection, AuthoringCorrectionKind,
    AuthoringFieldFact, AuthoringFieldPresence, AuthoringIdentity, AuthoringIdentityKind,
    AuthoringIngressClass, AuthoringIngressLimits, AuthoringInspection, AuthoringInspectionRequest,
    AuthoringInspectionState, AuthoringInstancePathSegment, AuthoringLiteral, AuthoringLiteralFact,
    AuthoringNavigation, AuthoringNavigationRequest, AuthoringNodeFact, AuthoringOccurrenceRole,
    AuthoringPathSegment, AuthoringSourceLocation, AuthoringValueKind, ContractDocument,
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticProjection, DiagnosticRetry,
    DiagnosticStage, DiagnosticV1, NativeActionDescriptor, NativeActionPayloadRequest,
    NativeActionRequest, PaleApi, PortableWorkspaceComparison, PortableWorkspaceCompilation,
    PortableWorkspaceDerivationRule, PortableWorkspaceDocument, PortableWorkspaceExplanation,
    PortableWorkspaceExplanationFrontier, PortableWorkspaceExplanationState, PortableWorkspaceFact,
    PortableWorkspaceImpact, PortableWorkspaceImpactState, PortableWorkspaceProvenance,
    PortableWorkspaceRequest, PortableWorkspaceSubject, PortableWorkspaceSubjectComparison,
    PortableWorkspaceSubjectKind, PortableWorkspaceSubjectValues, PortableWorkspaceValueComparison,
    StandaloneAuthoringRole,
};
use wasm_bindgen::prelude::*;

include!("surface/facade.rs");
include!("surface/diagnostic.rs");
include!("surface/native_action.rs");
include!("surface/contract.rs");
include!("surface/workspace.rs");
include!("surface/derivation.rs");
include!("surface/authoring.rs");
include!("surface/occurrences.rs");

#[cfg(test)]
#[path = "../tests/internal/mod.rs"]
mod tests;
