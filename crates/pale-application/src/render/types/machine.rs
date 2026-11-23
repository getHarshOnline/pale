//! Machine identities available to one render invocation.

use crate::terminal_adapters::ProviderBindingKey;

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    CapabilityIdV1, ExecutableIdentityDigest, NativeHarnessBinding, ProviderBindingRefV1,
    ResolvedProjectInstanceV1,
};

use super::RenderError;

/// Machine-local identities proven during this compilation.
///
/// Portable compilation has one explicit empty value. Normal compilation carries
/// identities that passed live probes. Removal reconstruction may carry the same
/// declared set without marking it verified, solely to reproduce the bytes and
/// manifest identity of surfaces already covered by trusted ownership state.
#[derive(Debug, Clone)]
pub(crate) struct MachineAdmission {
    verified: bool,
    provider_bindings: Vec<ProviderBindingRefV1>,
    capability_observations: BTreeMap<ProviderBindingKey, BTreeSet<CapabilityIdV1>>,
    executable_identity_digest: Option<ExecutableIdentityDigest>,
}

impl MachineAdmission {
    /// Compile portable surfaces without claiming any live machine proof.
    pub(crate) fn portable() -> Self {
        Self {
            verified: false,
            provider_bindings: Vec::new(),
            capability_observations: BTreeMap::new(),
            executable_identity_digest: None,
        }
    }

    /// Record the exact native harnesses proven by live machine probes.
    pub(crate) fn verified(
        provider_bindings: Vec<ProviderBindingRefV1>,
        capability_observations: BTreeMap<ProviderBindingKey, BTreeSet<CapabilityIdV1>>,
        executable_identity_digest: Option<ExecutableIdentityDigest>,
    ) -> Self {
        Self {
            verified: true,
            provider_bindings,
            capability_observations,
            executable_identity_digest,
        }
    }

    /// Reproduce a previously owned surface set without claiming live admission.
    pub(crate) fn reconstructed(
        provider_bindings: Vec<ProviderBindingRefV1>,
        capability_observations: BTreeMap<ProviderBindingKey, BTreeSet<CapabilityIdV1>>,
    ) -> Self {
        Self {
            verified: false,
            provider_bindings,
            capability_observations,
            executable_identity_digest: None,
        }
    }

    /// Executable identity admitted by the live machine pass, when configured.
    ///
    /// Absence means either that no surface executable was configured or that this is a
    /// portable/removal projection. It never substitutes for activation or an
    /// observed native effect.
    pub(crate) fn executable_identity_digest(&self) -> Option<&ExecutableIdentityDigest> {
        self.executable_identity_digest.as_ref()
    }

    /// Whether the admission came from live machine verification.
    pub(in crate::render) fn is_verified(&self) -> bool {
        self.verified
    }

    /// Capability carriers observed for one native harness during machine resolution.
    ///
    /// In live compilation these rows exist only after the exact surface executable and
    /// harness probes pass. Removal reconstruction may retain the same rows to
    /// reproduce already-owned bytes, while [`Self::is_verified`] remains false.
    /// A row proves neither invocation, evidence sufficiency, nor native effect.
    pub(in crate::render) fn capability_observations(
        &self,
        provider: &ProviderBindingRefV1,
    ) -> Option<&BTreeSet<CapabilityIdV1>> {
        self.capability_observations
            .get(&ProviderBindingKey::from(provider))
    }

    /// Resolve the exact native-harness binding included in this projection.
    ///
    /// Verified compilation includes only live-admitted bindings. Removal
    /// reconstruction includes the previously declared bindings solely to
    /// reproduce owned bytes, so callers must inspect [`Self::is_verified`]
    /// before describing a binding as admitted or runnable.
    pub(in crate::render) fn projected_harness_binding<'a>(
        &self,
        instance: &'a ResolvedProjectInstanceV1,
        provider: &ProviderBindingRefV1,
    ) -> Result<Option<&'a NativeHarnessBinding>, RenderError> {
        if !self.provider_bindings.contains(provider) {
            return Ok(None);
        }
        instance
            .native_harnesses
            .iter()
            .find(|binding| binding.provider_binding == *provider)
            .map(Some)
            .ok_or_else(|| {
                RenderError::invalid_input(
                    "machine admission",
                    format!(
                        "projection includes provider binding `{}`, but the resolved instance has no exact binding",
                        provider.provider_binding_id
                    ),
                )
            })
    }
}
