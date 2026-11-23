//! Operation exposure, requirement outcomes, and reasons for narrowing or blocking.
//!
//! These describe compiled configuration, not observed native readiness.

use pale_sdk::{
    EffectiveProfileNarrowingReason, EffectiveProfileOperationExposure,
    EffectiveProfilePolicyBlock, EffectiveProfileRequirementDisposition,
};
use pyo3::prelude::*;

/// Exact three-state operation exposure.
#[pyclass(name = "OperationExposure", eq, eq_int, frozen, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyOperationExposure {
    /// No private operation binding exists.
    Unbound,
    /// A binding exists with zero Package-owned requirements.
    SemanticOnly,
    /// A binding exists with Package-owned requirements.
    CapabilityBearing,
}

impl PyOperationExposure {
    pub(crate) fn from_api(value: EffectiveProfileOperationExposure) -> Self {
        match value {
            EffectiveProfileOperationExposure::Unbound => Self::Unbound,
            EffectiveProfileOperationExposure::SemanticOnly => Self::SemanticOnly,
            EffectiveProfileOperationExposure::CapabilityBearing => Self::CapabilityBearing,
        }
    }
}

/// Positive-set reason for narrowing.
#[pyclass(name = "NarrowingReason", eq, eq_int, frozen, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyNarrowingReason {
    /// Capability omitted.
    CapabilityNotPermitted,
    /// Effect omitted.
    EffectNotPermitted,
    /// Capability and effect omitted.
    CapabilityAndEffectNotPermitted,
}

impl PyNarrowingReason {
    fn from_api(value: EffectiveProfileNarrowingReason) -> Self {
        match value {
            EffectiveProfileNarrowingReason::CapabilityNotPermitted => Self::CapabilityNotPermitted,
            EffectiveProfileNarrowingReason::EffectNotPermitted => Self::EffectNotPermitted,
            EffectiveProfileNarrowingReason::CapabilityAndEffectNotPermitted => {
                Self::CapabilityAndEffectNotPermitted
            }
        }
    }
}

/// Exact policy dimension blocking a requirement.
#[pyclass(name = "PolicyBlock", eq, eq_int, frozen, skip_from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyPolicyBlock {
    /// Capability explicitly forbidden.
    CapabilityForbidden,
    /// Effect explicitly forbidden.
    EffectForbidden,
    /// Capability and effect explicitly forbidden.
    CapabilityAndEffectForbidden,
    /// Provider effect differs from the requirement.
    ProviderEffectMismatch,
    /// Provider lacks the required recipe contract.
    RecipeContractMismatch,
}

impl PyPolicyBlock {
    fn from_api(value: EffectiveProfilePolicyBlock) -> Self {
        match value {
            EffectiveProfilePolicyBlock::CapabilityForbidden => Self::CapabilityForbidden,
            EffectiveProfilePolicyBlock::EffectForbidden => Self::EffectForbidden,
            EffectiveProfilePolicyBlock::CapabilityAndEffectForbidden => {
                Self::CapabilityAndEffectForbidden
            }
            EffectiveProfilePolicyBlock::ProviderEffectMismatch => Self::ProviderEffectMismatch,
            EffectiveProfilePolicyBlock::RecipeContractMismatch => Self::RecipeContractMismatch,
        }
    }
}

/// A private provider satisfies the portable requirement.
#[pyclass(name = "SatisfiedRequirementDisposition", frozen)]
pub struct PySatisfiedRequirementDisposition {}

/// No private provider binding exists.
#[pyclass(name = "ProviderBindingAbsentRequirementDisposition", frozen)]
pub struct PyProviderBindingAbsentRequirementDisposition {}

/// A positive permitted set omitted the requirement.
#[pyclass(name = "NarrowedRequirementDisposition", frozen)]
pub struct PyNarrowedRequirementDisposition {
    reason: EffectiveProfileNarrowingReason,
}

#[pymethods]
impl PyNarrowedRequirementDisposition {
    /// Exact omitted policy dimension.
    #[getter]
    fn reason(&self) -> PyNarrowingReason {
        PyNarrowingReason::from_api(self.reason)
    }
}

/// Explicit policy or a present provider contradicts the requirement.
#[pyclass(name = "BlockedRequirementDisposition", frozen)]
pub struct PyBlockedRequirementDisposition {
    reason: EffectiveProfilePolicyBlock,
}

#[pymethods]
impl PyBlockedRequirementDisposition {
    /// Exact blocking dimension.
    #[getter]
    fn reason(&self) -> PyPolicyBlock {
        PyPolicyBlock::from_api(self.reason)
    }
}

#[derive(IntoPyObject)]
pub(crate) enum PyRequirementDisposition {
    #[pyo3(transparent)]
    Satisfied(PySatisfiedRequirementDisposition),
    #[pyo3(transparent)]
    ProviderBindingAbsent(PyProviderBindingAbsentRequirementDisposition),
    #[pyo3(transparent)]
    Narrowed(PyNarrowedRequirementDisposition),
    #[pyo3(transparent)]
    Blocked(PyBlockedRequirementDisposition),
}

impl PyRequirementDisposition {
    pub(crate) fn from_api(value: EffectiveProfileRequirementDisposition) -> Self {
        match value {
            EffectiveProfileRequirementDisposition::Satisfied => {
                Self::Satisfied(PySatisfiedRequirementDisposition {})
            }
            EffectiveProfileRequirementDisposition::ProviderBindingAbsent => {
                Self::ProviderBindingAbsent(PyProviderBindingAbsentRequirementDisposition {})
            }
            EffectiveProfileRequirementDisposition::Narrowed { reason } => {
                Self::Narrowed(PyNarrowedRequirementDisposition { reason })
            }
            EffectiveProfileRequirementDisposition::Blocked { reason } => {
                Self::Blocked(PyBlockedRequirementDisposition { reason })
            }
        }
    }
}
