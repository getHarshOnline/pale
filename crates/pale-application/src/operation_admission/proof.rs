//! Semantic proofs admitted after enclosing artifact bytes have been verified.

use pale_core::records::{
    ArtifactDigest, CapabilityProvider, EffectClass, ExecutionPathWitnessV1, FailurePosture,
    LogicalRepositoryIdV1, OperationBindingDigest, OperationCapabilityContractV1,
    OperationCatalogDigest, OperationCatalogEntryV1, OperationContractCatalogV1,
    OperationContractDigest, OperationContractMechanicsV1, OperationContractV1,
    OperationRouteBindingV1, PackageResourceRefV1, ProfileDigest, SealedArtifactRef,
};
use serde::de::DeserializeOwned;

use crate::hooks::HookEvent;
use crate::terminal_adapters::TerminalAdapterDescriptor;

use super::{verify, OperationAdmissionError};

/// Verify the failure carrier for one exact operation requirement.
///
/// Pale-native actions are compute-only and return their typed outcome directly
/// to Pale. Host-native and connector requirements instead depend on their
/// route-local host tools and the selected harness's failed-tool event. An
/// unavailable provider cannot start and is governed by its separate
/// requirement policy, so it needs no hypothetical failure carrier here.
fn admit_failure_observation(
    operation: &PackageResourceRefV1,
    capability: &OperationCapabilityContractV1,
    adapter: TerminalAdapterDescriptor,
) -> Result<(), OperationAdmissionError> {
    if capability.failure_posture == FailurePosture::FailOpen || !capability.available() {
        return Ok(());
    }
    let provider = capability
        .provider_state
        .provider()
        .expect("an available capability has a bound provider");
    let observable = match (provider, capability.effect_class) {
        (CapabilityProvider::NativeRust, EffectClass::Compute) => {
            capability.provider_state.native_action().is_some()
        }
        (CapabilityProvider::NativeRust, _) => false,
        (CapabilityProvider::HostNative | CapabilityProvider::Connector, _) => {
            !capability.provider_state.host_tools().is_empty()
                && crate::hooks::harness_supports_event(adapter, HookEvent::PostToolUseFailure)
        }
    };
    if observable {
        Ok(())
    } else {
        Err(OperationAdmissionError::FailureObservationUnavailable {
            operation: operation.clone(),
            requirement: capability.requirement_id.clone(),
            provider,
            effect: capability.effect_class,
            adapter,
        })
    }
}

/// An owned catalog after the binding, canonical self-digests, profile,
/// repository, native harness, and catalog identity agree.
///
/// Construction verifies the binding bytes against the caller's root reference,
/// then the catalog bytes against the reference inside that binding. This
/// one-process snapshot does not observe later file replacement. Leases from
/// older snapshots are rejected when their stored binding or catalog digest differs.
pub struct VerifiedOperationBundle {
    catalog: OperationContractCatalogV1,
    profile_digest: ProfileDigest,
    repository_id: LogicalRepositoryIdV1,
    execution_path_witness: ExecutionPathWitnessV1,
    adapter: TerminalAdapterDescriptor,
    binding_digest: OperationBindingDigest,
    catalog_digest: OperationCatalogDigest,
}

impl VerifiedOperationBundle {
    /// Admit exact binding and catalog bytes chained from one sealed root reference.
    ///
    /// # Errors
    ///
    /// Rejects byte/digest or strict-decoding mismatches, unsupported or
    /// internally inconsistent binding/catalog records, and a sealed path
    /// context that differs from the expected profile, repository, or path.
    pub fn try_from_sealed_bytes(
        binding_reference: &SealedArtifactRef,
        binding_bytes: &[u8],
        catalog_bytes: &[u8],
        expected_profile: &ProfileDigest,
        expected_repository: &LogicalRepositoryIdV1,
        expected_path_witness: &ExecutionPathWitnessV1,
        adapter: TerminalAdapterDescriptor,
    ) -> Result<Self, OperationAdmissionError> {
        let binding: OperationRouteBindingV1 = decode_sealed(
            binding_reference,
            binding_bytes,
            "operation-route-binding.v1",
        )?;
        let catalog: OperationContractCatalogV1 =
            decode_sealed(&binding.catalog, catalog_bytes, "operation-catalog.v1")?;
        verify::bundle(&binding, &catalog)?;
        if &binding.profile_digest != expected_profile
            || &binding.repository_id != expected_repository
            || &binding.execution_path_witness != expected_path_witness
        {
            return Err(OperationAdmissionError::RouteContextMismatch);
        }
        let binding_digest = binding.binding_digest.clone();
        let catalog_digest = catalog.catalog_digest.clone();
        Ok(Self {
            catalog,
            profile_digest: binding.profile_digest,
            repository_id: binding.repository_id,
            execution_path_witness: binding.execution_path_witness,
            adapter,
            binding_digest,
            catalog_digest,
        })
    }

    /// Catalog covered by `catalog_digest` and bound by the path record.
    pub(super) fn catalog(&self) -> &OperationContractCatalogV1 {
        &self.catalog
    }

    /// Digest of the path-local binding that admitted this bundle.
    pub(super) fn binding_digest(&self) -> &OperationBindingDigest {
        &self.binding_digest
    }

    /// Digest of the operation catalog covered by the binding.
    pub(super) fn catalog_digest(&self) -> &OperationCatalogDigest {
        &self.catalog_digest
    }

    /// Profile identity against which the bundle was admitted.
    pub(super) fn profile_digest(&self) -> &ProfileDigest {
        &self.profile_digest
    }

    /// Repository identity against which the bundle was admitted.
    pub(super) fn repository_id(&self) -> &LogicalRepositoryIdV1 {
        &self.repository_id
    }

    /// Exact neutral execution path sealed by the admitted binding.
    pub(super) fn execution_path_witness(&self) -> &ExecutionPathWitnessV1 {
        &self.execution_path_witness
    }

    /// Resolve the sealed contract reference covered by this bundle's catalog digest.
    ///
    /// # Errors
    ///
    /// Rejects an operation identity absent from the admitted catalog.
    pub fn contract_reference(
        &self,
        operation_id: &PackageResourceRefV1,
    ) -> Result<&SealedArtifactRef, OperationAdmissionError> {
        Ok(&verify::entry_for_id(&self.catalog, operation_id)?.contract)
    }

    /// Verify exact contract bytes against the catalog reference and compact identity.
    ///
    /// The result binds the contract digest to this binding and catalog proof;
    /// activation and tool admission require that result.
    ///
    /// # Errors
    ///
    /// Rejects missing catalog entries, byte/digest or strict-decoding
    /// mismatches, unsupported contract schemas, and compact-entry/contract or
    /// route-enforcement inconsistencies.
    pub fn admit_contract_bytes(
        &self,
        operation_id: &PackageResourceRefV1,
        bytes: &[u8],
    ) -> Result<AdmittedOperation<'_>, OperationAdmissionError> {
        let entry = verify::entry_for_id(&self.catalog, operation_id)?;
        let contract: OperationContractV1 =
            decode_sealed(&entry.contract, bytes, "operation-contract.v1")?;
        verify::entry_contract(entry, &contract)?;
        if let OperationContractMechanicsV1::CapabilityBearing { capabilities } =
            &contract.mechanics
        {
            for capability in capabilities {
                admit_failure_observation(&contract.id, capability, self.adapter)?;
            }
        }
        let contract_digest = contract.contract_digest.clone();
        Ok(AdmittedOperation {
            bundle: self,
            entry,
            contract,
            contract_digest,
        })
    }
}

fn decode_sealed<T: DeserializeOwned>(
    reference: &SealedArtifactRef,
    bytes: &[u8],
    schema_id: &str,
) -> Result<T, OperationAdmissionError> {
    if ArtifactDigest::hash_bytes(bytes) != reference.sha256 {
        return Err(OperationAdmissionError::ArtifactDigestMismatch {
            artifact: reference.rel_path.clone(),
        });
    }
    let value = pale_core::canon::parse_strict(bytes).map_err(|_| {
        OperationAdmissionError::MalformedArtifact {
            artifact: reference.rel_path.clone(),
        }
    })?;
    pale_contracts::validate_value(&value, schema_id).map_err(|_| {
        OperationAdmissionError::MalformedArtifact {
            artifact: reference.rel_path.clone(),
        }
    })?;
    serde_json::from_value(value).map_err(|_| OperationAdmissionError::MalformedArtifact {
        artifact: reference.rel_path.clone(),
    })
}

/// A contract bound to verified binding, catalog, entry, and contract digests.
///
/// It is immutable for one process. A lease becomes stale when any stored
/// binding, catalog, or contract digest differs. Tool admission requires this
/// proof and performs no canonical record-digest recomputation. Activation
/// additionally requires [`ActivationReadyOperation`].
pub struct AdmittedOperation<'a> {
    pub(super) bundle: &'a VerifiedOperationBundle,
    pub(super) entry: &'a OperationCatalogEntryV1,
    pub(super) contract: OperationContractV1,
    pub(super) contract_digest: OperationContractDigest,
}

/// An admitted operation whose complete model-facing closure matches its sealed bytes.
pub struct ActivationReadyOperation<'a> {
    pub(super) operation: AdmittedOperation<'a>,
}

impl<'a> AdmittedOperation<'a> {
    /// Operation identity covered by the admitted contract digest.
    pub fn id(&self) -> &PackageResourceRefV1 {
        &self.contract.id
    }

    /// Immutable contract covered by the admitted contract digest.
    pub fn contract(&self) -> &OperationContractV1 {
        &self.contract
    }

    /// Verify the skill, optional invocation policy, and every semantic dependency.
    ///
    /// # Errors
    ///
    /// Rejects a missing, extra, or byte-mismatched member of the contract's
    /// complete model-facing activation closure.
    pub fn admit_activation_content<'content>(
        self,
        skill: &[u8],
        invocation_policy: Option<&[u8]>,
        semantic_dependencies: impl IntoIterator<Item = &'content [u8]>,
    ) -> Result<ActivationReadyOperation<'a>, OperationAdmissionError> {
        verify_content(&self.contract.skill, skill)?;
        match (&self.contract.invocation_policy, invocation_policy) {
            (Some(reference), Some(bytes)) => verify_content(reference, bytes)?,
            (None, None) => {}
            (Some(reference), None) => {
                return Err(OperationAdmissionError::SemanticClosureMismatch {
                    artifact: reference.rel_path.clone(),
                });
            }
            (None, Some(_)) => {
                return Err(OperationAdmissionError::SemanticClosureMismatch {
                    artifact: "invocation-policy".into(),
                });
            }
        }
        let mut content = semantic_dependencies.into_iter();
        for dependency in &self.contract.semantic_dependencies {
            let bytes =
                content
                    .next()
                    .ok_or_else(|| OperationAdmissionError::SemanticClosureMismatch {
                        artifact: dependency.artifact.rel_path.clone(),
                    })?;
            verify_content(&dependency.artifact, bytes)?;
        }
        if content.next().is_some() {
            return Err(OperationAdmissionError::SemanticClosureMismatch {
                artifact: "unexpected-semantic-dependency".into(),
            });
        }
        Ok(ActivationReadyOperation { operation: self })
    }
}

fn verify_content(
    reference: &SealedArtifactRef,
    bytes: &[u8],
) -> Result<(), OperationAdmissionError> {
    if ArtifactDigest::hash_bytes(bytes) == reference.sha256 {
        Ok(())
    } else {
        Err(OperationAdmissionError::ArtifactDigestMismatch {
            artifact: reference.rel_path.clone(),
        })
    }
}

impl ActivationReadyOperation<'_> {
    /// Operation identity whose complete activation closure was admitted.
    pub fn id(&self) -> &PackageResourceRefV1 {
        self.operation.id()
    }
}
