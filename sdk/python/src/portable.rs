//! Portable and effect-free Python projections.
//!
//! Contract, diagnostic, workspace compilation, derivation, native-action, and
//! [`PalePython`] façade fragments compose this single private projection.
//! Keeping one module boundary avoids exposing internal Python wrapper topology.

use std::sync::Arc;

use pale_sdk::{
    ContractDocument, DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticProjection,
    DiagnosticRetry, DiagnosticStage, DiagnosticV1, NativeActionDescriptor,
    NativeActionPayloadRequest, NativeActionRequest, PortableWorkspaceCompilation,
    PortableWorkspaceDerivationRule, PortableWorkspaceDocument, PortableWorkspaceExplanation,
    PortableWorkspaceExplanationFrontier, PortableWorkspaceExplanationState, PortableWorkspaceFact,
    PortableWorkspaceImpact, PortableWorkspaceImpactState, PortableWorkspaceProvenance,
    PortableWorkspaceRequest, PortableWorkspaceSubject, PortableWorkspaceSubjectKind,
    WorkspaceModuleCandidate,
};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::error::diagnostic_error;
use crate::PalePython;

include!("portable/diagnostic.rs");
include!("portable/contract.rs");
include!("portable/native_action.rs");
include!("portable/workspace_compilation.rs");
include!("portable/derivation.rs");
include!("portable/subject_comparison.rs");
include!("portable/comparison_subjects.rs");
include!("portable/proof_dependencies.rs");
include!("portable/facade.rs");
