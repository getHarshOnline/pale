//! Participants, responsibilities and ceilings retained by profile rows.
//!
//! Concrete participant and responsibility variants carry only their own
//! fields. These describe compiled configuration, not native readiness.

use pale_sdk::{
    EffectiveProfileEffectClass, EffectiveProfileParticipant, EffectiveProfileProviderBindingRef,
    EffectiveProfileRepositoryAccess, EffectiveProfileResponsibility,
};
use pyo3::prelude::*;

/// Portable effect ceiling, without authority to perform the effect.
#[pyclass(
    name = "EffectiveProfileEffectClass",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEffectiveProfileEffectClass {
    /// Pure computation.
    Compute,
    /// Non-mutating read.
    Read,
    /// Local execution.
    Execute,
    /// Local mutation.
    Write,
    /// Remote read.
    Network,
    /// Outward-facing write.
    Publish,
}

impl PyEffectiveProfileEffectClass {
    pub(crate) fn from_api(value: EffectiveProfileEffectClass) -> Self {
        match value {
            EffectiveProfileEffectClass::Compute => Self::Compute,
            EffectiveProfileEffectClass::Read => Self::Read,
            EffectiveProfileEffectClass::Execute => Self::Execute,
            EffectiveProfileEffectClass::Write => Self::Write,
            EffectiveProfileEffectClass::Network => Self::Network,
            EffectiveProfileEffectClass::Publish => Self::Publish,
        }
    }
}

/// Repository access ceiling before independent effect admission.
#[pyclass(
    name = "EffectiveProfileRepositoryAccess",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEffectiveProfileRepositoryAccess {
    /// Read-only access.
    Read,
    /// Mutation may be considered by later admission.
    Mutate,
}

impl PyEffectiveProfileRepositoryAccess {
    pub(crate) fn from_api(value: EffectiveProfileRepositoryAccess) -> Self {
        match value {
            EffectiveProfileRepositoryAccess::Read => Self::Read,
            EffectiveProfileRepositoryAccess::Mutate => Self::Mutate,
        }
    }
}

/// Contract-facing interface assigned a responsibility.
#[pyclass(name = "EffectiveProfileInterfaceParticipant", frozen, get_all)]
pub struct PyEffectiveProfileInterfaceParticipant {
    /// Stable interface identity.
    interface_id: String,
}

/// Native application assigned a responsibility.
#[pyclass(name = "EffectiveProfileApplicationParticipant", frozen, get_all)]
pub struct PyEffectiveProfileApplicationParticipant {
    /// Stable application identity.
    application_id: String,
}

/// Participant from the initial-v1 `engine` wire variant assigned a responsibility.
#[pyclass(name = "EffectiveProfileEngineParticipant", frozen, get_all)]
pub struct PyEffectiveProfileEngineParticipant {
    /// Stable identity carried by the `engine` wire variant.
    engine_id: String,
}

/// Neutral capability assigned a responsibility.
#[pyclass(name = "EffectiveProfileCapabilityParticipant", frozen, get_all)]
pub struct PyEffectiveProfileCapabilityParticipant {
    /// Stable capability identity.
    capability_id: String,
}

/// Terminal leaf assigned a responsibility.
#[pyclass(name = "EffectiveProfileTerminalLeafParticipant", frozen, get_all)]
pub struct PyEffectiveProfileTerminalLeafParticipant {
    /// Stable terminal-leaf identity.
    terminal_leaf_id: String,
}

/// Plugin assigned a responsibility.
#[pyclass(name = "EffectiveProfilePluginParticipant", frozen, get_all)]
pub struct PyEffectiveProfilePluginParticipant {
    /// Stable plugin identity.
    plugin_id: String,
}

/// Skill assigned a responsibility.
#[pyclass(name = "EffectiveProfileSkillParticipant", frozen, get_all)]
pub struct PyEffectiveProfileSkillParticipant {
    /// Stable skill identity.
    skill_id: String,
}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileParticipant {
    Interface(PyEffectiveProfileInterfaceParticipant),
    Application(PyEffectiveProfileApplicationParticipant),
    Engine(PyEffectiveProfileEngineParticipant),
    Capability(PyEffectiveProfileCapabilityParticipant),
    TerminalLeaf(PyEffectiveProfileTerminalLeafParticipant),
    Plugin(PyEffectiveProfilePluginParticipant),
    Skill(PyEffectiveProfileSkillParticipant),
}

impl PyEffectiveProfileParticipant {
    pub(crate) fn from_api(value: EffectiveProfileParticipant) -> Self {
        match value {
            EffectiveProfileParticipant::Interface { interface_id } => {
                Self::Interface(PyEffectiveProfileInterfaceParticipant { interface_id })
            }
            EffectiveProfileParticipant::Application { application_id } => {
                Self::Application(PyEffectiveProfileApplicationParticipant { application_id })
            }
            EffectiveProfileParticipant::Engine { engine_id } => {
                Self::Engine(PyEffectiveProfileEngineParticipant { engine_id })
            }
            EffectiveProfileParticipant::Capability { capability_id } => {
                Self::Capability(PyEffectiveProfileCapabilityParticipant { capability_id })
            }
            EffectiveProfileParticipant::TerminalLeaf { terminal_leaf_id } => {
                Self::TerminalLeaf(PyEffectiveProfileTerminalLeafParticipant { terminal_leaf_id })
            }
            EffectiveProfileParticipant::Plugin { plugin_id } => {
                Self::Plugin(PyEffectiveProfilePluginParticipant { plugin_id })
            }
            EffectiveProfileParticipant::Skill { skill_id } => {
                Self::Skill(PyEffectiveProfileSkillParticipant { skill_id })
            }
        }
    }
}

/// Human or agent communication responsibility.
#[pyclass(name = "EffectiveProfileCommunicationResponsibility", frozen)]
pub struct PyEffectiveProfileCommunicationResponsibility {}

/// Organizational planning, delegation or acceptance responsibility.
#[pyclass(name = "EffectiveProfileOrganizationalWorkResponsibility", frozen)]
pub struct PyEffectiveProfileOrganizationalWorkResponsibility {}

/// Customer-relationship or other business-owned state responsibility.
#[pyclass(name = "EffectiveProfileCrmBusinessStateResponsibility", frozen)]
pub struct PyEffectiveProfileCrmBusinessStateResponsibility {}

/// Engineering work through its native execution owner.
#[pyclass(name = "EffectiveProfileEngineeringExecutionResponsibility", frozen)]
pub struct PyEffectiveProfileEngineeringExecutionResponsibility {}

/// Context or memory resolution without ownership transfer.
#[pyclass(name = "EffectiveProfileContextMemoryResponsibility", frozen)]
pub struct PyEffectiveProfileContextMemoryResponsibility {}

/// Terminal harness or model-backed execution responsibility.
#[pyclass(
    name = "EffectiveProfileTerminalHarnessExecutionResponsibility",
    frozen
)]
pub struct PyEffectiveProfileTerminalHarnessExecutionResponsibility {}

/// Publication through a native event or outbox boundary.
#[pyclass(name = "EffectiveProfileNativeEventPublicationResponsibility", frozen)]
pub struct PyEffectiveProfileNativeEventPublicationResponsibility {}

/// Native effect followed by owner read-back.
#[pyclass(name = "EffectiveProfileNativeEffectReadBackResponsibility", frozen)]
pub struct PyEffectiveProfileNativeEffectReadBackResponsibility {}

/// Separately governed extension responsibility.
#[pyclass(name = "EffectiveProfileExtensionResponsibility", frozen, get_all)]
pub struct PyEffectiveProfileExtensionResponsibility {
    /// Namespace owning the extension vocabulary.
    namespace_id: String,
    /// Contract identity within that namespace.
    contract_id: String,
    /// Exact admitted schema member.
    schema_id: String,
    /// SHA-256 of the exact schema bytes.
    schema_sha256: String,
}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileResponsibility {
    Communication(PyEffectiveProfileCommunicationResponsibility),
    OrganizationalWork(PyEffectiveProfileOrganizationalWorkResponsibility),
    CrmBusinessState(PyEffectiveProfileCrmBusinessStateResponsibility),
    EngineeringExecution(PyEffectiveProfileEngineeringExecutionResponsibility),
    ContextMemory(PyEffectiveProfileContextMemoryResponsibility),
    TerminalHarnessExecution(PyEffectiveProfileTerminalHarnessExecutionResponsibility),
    NativeEventPublication(PyEffectiveProfileNativeEventPublicationResponsibility),
    NativeEffectReadBack(PyEffectiveProfileNativeEffectReadBackResponsibility),
    Extension(PyEffectiveProfileExtensionResponsibility),
}

impl PyEffectiveProfileResponsibility {
    pub(crate) fn from_api(value: EffectiveProfileResponsibility) -> Self {
        match value {
            EffectiveProfileResponsibility::Communication => {
                Self::Communication(PyEffectiveProfileCommunicationResponsibility {})
            }
            EffectiveProfileResponsibility::OrganizationalWork => {
                Self::OrganizationalWork(PyEffectiveProfileOrganizationalWorkResponsibility {})
            }
            EffectiveProfileResponsibility::CrmBusinessState => {
                Self::CrmBusinessState(PyEffectiveProfileCrmBusinessStateResponsibility {})
            }
            EffectiveProfileResponsibility::EngineeringExecution => {
                Self::EngineeringExecution(PyEffectiveProfileEngineeringExecutionResponsibility {})
            }
            EffectiveProfileResponsibility::ContextMemory => {
                Self::ContextMemory(PyEffectiveProfileContextMemoryResponsibility {})
            }
            EffectiveProfileResponsibility::TerminalHarnessExecution => {
                Self::TerminalHarnessExecution(
                    PyEffectiveProfileTerminalHarnessExecutionResponsibility {},
                )
            }
            EffectiveProfileResponsibility::NativeEventPublication => Self::NativeEventPublication(
                PyEffectiveProfileNativeEventPublicationResponsibility {},
            ),
            EffectiveProfileResponsibility::NativeEffectReadBack => {
                Self::NativeEffectReadBack(PyEffectiveProfileNativeEffectReadBackResponsibility {})
            }
            EffectiveProfileResponsibility::Extension {
                namespace_id,
                contract_id,
                schema_id,
                schema_sha256,
            } => Self::Extension(PyEffectiveProfileExtensionResponsibility {
                namespace_id,
                contract_id,
                schema_id,
                schema_sha256,
            }),
        }
    }
}

/// Exact terminal provider-binding coordinate; not native availability evidence.
#[pyclass(name = "EffectiveProfileProviderBindingRef", frozen)]
pub struct PyEffectiveProfileProviderBindingRef {
    inner: EffectiveProfileProviderBindingRef,
}

impl PyEffectiveProfileProviderBindingRef {
    pub(crate) fn from_api(inner: EffectiveProfileProviderBindingRef) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEffectiveProfileProviderBindingRef {
    /// Stable binding locator.
    #[getter]
    pub fn provider_binding_id(&self) -> &str {
        &self.inner.provider_binding_id
    }

    /// Identity of the complete binding body.
    #[getter]
    pub fn provider_binding_identity(&self) -> &str {
        &self.inner.provider_binding_identity
    }
}
