//! Opaque handoff from semantic plan admission to filesystem effect staging.
//!
//! The two capabilities in this module deliberately expose no paths, records,
//! rendered payloads, or mutable access. Host composition may carry them across
//! the API-to-adapter boundary, but only the owning admission and adapter crates
//! can use their hidden record projections.

use std::collections::BTreeMap;
use std::sync::Arc;

use pale_core::admission::AdmittedPlan;
use pale_core::records::{MachineLocalEffectV1, MaterializationContractIdentity, PlanKind};

/// Bounded regular-file and ancestor observation supplied by a filesystem owner.
///
/// This process-local input contains no mutation authority. Consumers validate
/// the byte/identity relationship; an effect adapter must reobserve it before
/// publication. It is shared by runtime and source-guidance planning.
#[derive(Clone, PartialEq, Eq)]
pub struct MachineLocalFileObservation {
    /// Descriptor-bound root and its canonical location.
    pub root: pale_core::records::MachineLocalRootIdentity,
    /// Observed file metadata; `None` denotes absence.
    pub prior: Option<pale_core::records::ExistingMachineLocalFileV1>,
    /// Exact bytes, present exactly when [`Self::prior`] is present.
    pub bytes: Option<Vec<u8>>,
    /// Existing ancestor identities, in shallowest-first order.
    pub retained_parent_directories: Vec<pale_core::records::MachineLocalDirectoryIdentity>,
    /// Missing ancestors a proposed write may need to create.
    pub missing_parent_directories: Vec<String>,
}

/// Opaque host observation proposed for machine-local effect binding.
///
/// Construction alone grants no authority. The application planning owner must
/// validate and bind the carried record before it can enter an
/// [`AdmittedMaterializationEffect`]. Debug output intentionally reveals no
/// machine path or target content.
pub struct MachineLocalEffectCandidate {
    record: MachineLocalEffectV1,
}

impl std::fmt::Debug for MachineLocalEffectCandidate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MachineLocalEffectCandidate")
            .finish_non_exhaustive()
    }
}

impl MachineLocalEffectCandidate {
    /// Seal one host-composed record behind the path-redacting boundary.
    ///
    /// This constructor does not validate or admit the record. Its only purpose
    /// is to prevent composition leaves from inspecting or translating the
    /// record after they hand it to the API admission owner.
    #[doc(hidden)]
    #[must_use]
    pub fn from_host_observation(record: MachineLocalEffectV1) -> Self {
        Self { record }
    }

    /// Consume the opaque candidate at the semantic admission boundary.
    #[doc(hidden)]
    #[must_use]
    pub fn into_record(self) -> MachineLocalEffectV1 {
        self.record
    }
}

/// Shared rendered payload retained for one admitted materialization effect.
pub struct MaterializationPayload(Arc<String>);

impl MaterializationPayload {
    /// Preserve the compiler-owned allocation across the effect boundary.
    #[doc(hidden)]
    #[must_use]
    pub fn from_shared(content: Arc<String>) -> Self {
        Self(content)
    }
}

impl AsRef<str> for MaterializationPayload {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

/// Saved-plan authority admitted by the API and consumed by the filesystem adapter.
///
/// The capability is path-free at its public surface. It cannot be cloned,
/// serialized, decoded, or constructed from public wire data. Its hidden parts
/// remain available only for the in-workspace adapter handoff and carry no OS
/// handle, transaction, activation, or effect by themselves.
pub struct AdmittedMaterializationEffect {
    plan: AdmittedPlan,
    payloads: BTreeMap<String, MaterializationPayload>,
    contract_identity: MaterializationContractIdentity,
}

impl std::fmt::Debug for AdmittedMaterializationEffect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedMaterializationEffect")
            .field("kind", &self.plan.kind())
            .field("payload_count", &self.payloads.len())
            .finish_non_exhaustive()
    }
}

impl AdmittedMaterializationEffect {
    /// Bind an admitted plan to its compiler-retained payloads and contract identity.
    #[doc(hidden)]
    #[must_use]
    pub fn from_api_admission(
        plan: AdmittedPlan,
        payloads: BTreeMap<String, MaterializationPayload>,
        contract_identity: MaterializationContractIdentity,
    ) -> Self {
        Self {
            plan,
            payloads,
            contract_identity,
        }
    }

    /// Operation class admitted by the saved-plan equality gate.
    #[must_use]
    pub fn kind(&self) -> PlanKind {
        self.plan.kind()
    }

    /// Borrow the admitted proof for the owning filesystem adapter.
    #[doc(hidden)]
    pub fn admitted_plan(&self) -> &AdmittedPlan {
        &self.plan
    }

    /// Borrow compiler-retained payloads for the owning filesystem adapter.
    #[doc(hidden)]
    #[must_use]
    pub fn payloads(&self) -> &BTreeMap<String, MaterializationPayload> {
        &self.payloads
    }

    /// Borrow the exact contract identity for the owning filesystem adapter.
    #[doc(hidden)]
    #[must_use]
    pub const fn contract_identity(&self) -> &MaterializationContractIdentity {
        &self.contract_identity
    }
}
