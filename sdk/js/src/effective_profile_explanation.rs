//! Complete evidence returned by an existing retained profile explanation.
//!
//! The Rust SDK selects the explanation and owns its opaque handle. This
//! module projects that value without deriving evidence from page bytes.

use napi_derive::napi;
use pale_sdk::{
    EffectiveProfileExplanation as ApiExplanation,
    EffectiveProfileExplanationEvidence as ApiEvidence,
};

use crate::{
    EffectiveProfileDeclaredSelection, EffectiveProfileFinding, EffectiveProfileNativeRealization,
    EffectiveProfilePathReason, EffectiveProfileProviderDisposition, EffectiveProfileSelection,
    OperationExposure, RequirementDisposition,
};

/// One retained explanation and its complete typed evidence.
#[napi(object, object_from_js = false)]
pub struct EffectiveProfileExplanation {
    /// Opaque SDK-owned identity matching the requested handle.
    pub handle: String,
    /// Complete evidence with required fields selected by its literal kind.
    pub evidence: EffectiveProfileExplanationEvidence,
}

impl EffectiveProfileExplanation {
    pub(crate) fn from_api(value: ApiExplanation) -> Self {
        Self {
            handle: value.handle.as_str().to_owned(),
            evidence: EffectiveProfileExplanationEvidence::from_api(value.evidence),
        }
    }
}

/// Exact SDK evidence copied into a discriminated output value.
///
/// Ordered reason lists, including empty lists, are preserved. Configuration
/// does not authorize effects or establish machine/process readiness.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfileExplanationEvidence {
    /// Exact declaration before closure expansion.
    DeclaredSelection {
        /// Normalized unexpanded selection.
        selection: EffectiveProfileDeclaredSelection,
    },
    /// Admitted responsibility and its composition proof.
    Responsibility {
        /// Exact responsibility contract.
        contract_id: String,
        /// Selected path carrying the contract.
        execution_path_id: String,
        /// Ordered composition reasons, possibly empty.
        reason_steps: Vec<EffectiveProfilePathReason>,
    },
    /// A member of the admitted effective selection.
    EffectiveSelection {
        /// Exact selected coordinate.
        selection: EffectiveProfileSelection,
    },
    /// A selected execution path and its composition proof.
    ExecutionPath {
        /// Stable selected-path identity.
        selection_id: String,
        /// Ordered composition reasons, possibly empty.
        reason_steps: Vec<EffectiveProfilePathReason>,
    },
    /// Independent operation exposure and native evidence.
    OperationClassification {
        /// Application-owned exposure.
        exposure: OperationExposure,
        /// Independently qualified native evidence.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// Independent portable, provider and native classifications.
    CapabilityClassification {
        /// Portable requirement outcome.
        requirement_disposition: RequirementDisposition,
        /// Private provider-selection outcome.
        provider_disposition: EffectiveProfileProviderDisposition,
        /// Independently qualified native evidence.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// One exact application-owned finding.
    FindingClassification {
        /// Structured finding without added inference.
        finding: EffectiveProfileFinding,
    },
    /// Separately qualified owner-native realization.
    MachineRealization {
        /// Independent native-evidence state.
        realization: EffectiveProfileNativeRealization,
    },
    /// Retained configuration evidence requiring no inference.
    Configuration {},
}

impl EffectiveProfileExplanationEvidence {
    pub(crate) fn from_api(value: ApiEvidence) -> Self {
        match value {
            ApiEvidence::DeclaredSelection { selection } => Self::DeclaredSelection {
                selection: EffectiveProfileDeclaredSelection::from_api(&selection),
            },
            ApiEvidence::Responsibility {
                contract_id,
                execution_path_id,
                reason_steps,
            } => Self::Responsibility {
                contract_id,
                execution_path_id,
                reason_steps: reason_steps
                    .iter()
                    .map(EffectiveProfilePathReason::from_api)
                    .collect(),
            },
            ApiEvidence::EffectiveSelection { selection } => Self::EffectiveSelection {
                selection: EffectiveProfileSelection::from_api(&selection),
            },
            ApiEvidence::ExecutionPath {
                selection_id,
                reason_steps,
            } => Self::ExecutionPath {
                selection_id,
                reason_steps: reason_steps
                    .iter()
                    .map(EffectiveProfilePathReason::from_api)
                    .collect(),
            },
            ApiEvidence::OperationClassification {
                exposure,
                native_realization,
            } => Self::OperationClassification {
                exposure: OperationExposure::from_api(exposure),
                native_realization: EffectiveProfileNativeRealization::from_api(native_realization),
            },
            ApiEvidence::CapabilityClassification {
                requirement_disposition,
                provider_disposition,
                native_realization,
            } => Self::CapabilityClassification {
                requirement_disposition: RequirementDisposition::from_api(requirement_disposition),
                provider_disposition: EffectiveProfileProviderDisposition::from_api(
                    provider_disposition,
                ),
                native_realization: EffectiveProfileNativeRealization::from_api(native_realization),
            },
            ApiEvidence::FindingClassification { finding } => Self::FindingClassification {
                finding: EffectiveProfileFinding::from_api(&finding),
            },
            ApiEvidence::MachineRealization { realization } => Self::MachineRealization {
                realization: EffectiveProfileNativeRealization::from_api(realization),
            },
            ApiEvidence::Configuration => Self::Configuration {},
        }
    }
}
