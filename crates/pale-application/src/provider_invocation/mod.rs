//! Call a provider for one operation, check its reply, and work out whether that
//! operation is ready to run.
//!
//! This module builds on `compile` and `observation`; `resident` builds on it.
//!
//! [`PreparedProviderInvocation`] is the only ordinary provider call path. It
//! screens out effectful requests, invokes one exact [`pale_ports::ProviderInvocationPort`],
//! compares the complete response coordinate, and contextually admits the
//! existing signed result or safe-failure contracts. [`ProviderInvocationOperationReadiness`]
//! separately derives readiness for one operation and one compiled generation;
//! it never promotes a sibling capability or whole provider.
//!
//! Admission checks a request or returned evidence against independently
//! selected identity, authority, policy, and freshness. It neither executes
//! work nor grants external authority.
//!
//! Synthetic conformance fixtures prove the neutral laws but create no provider
//! admission, native readiness, runtime activation, or effect authority.

mod admission;
mod readiness;

pub use admission::{
    AdmittedProviderInvocation, AdmittedProviderInvocationFailure,
    AdmittedProviderInvocationOutcome, AdmittedProviderInvocationSuccess,
    PreparedProviderInvocation, ProviderInvocationAdmissionContext,
    ProviderInvocationAdmissionError, ProviderInvocationEvidenceAdmission,
    ProviderInvocationSafeFailureProof, ProviderInvocationUnavailableAttempt,
};
pub(crate) use admission::{OwnedProviderInvocationAdmission, ProviderInvocationAttemptAdmission};
pub use readiness::{
    ProviderInvocationAuthorityCoordinate, ProviderInvocationBindingCoordinate,
    ProviderInvocationBoundCapability, ProviderInvocationCapabilityReadiness,
    ProviderInvocationCapabilitySet, ProviderInvocationComponent,
    ProviderInvocationDependencyCoordinate, ProviderInvocationDependencySet,
    ProviderInvocationOperationExposure, ProviderInvocationOperationReadiness,
    ProviderInvocationReadinessError, ProviderInvocationReadinessState,
    ProviderInvocationRequirementState,
};

#[cfg(test)]
#[path = "../../tests/internal/provider_invocation/mod.rs"]
mod tests;
