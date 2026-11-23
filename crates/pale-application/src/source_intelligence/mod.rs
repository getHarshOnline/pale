//! Keep and query a provider-built index of a repository's source, and record
//! which returned evidence a consumer used.
//!
//! This module builds on `semantic`; no other module in this crate builds on it.
//!
//! The subsystem stores deletable provider-derived structure, never repository
//! source truth. [`SourceIndexAdministrator`] owns generation transitions;
//! [`SourceIntelligenceQueryService`] can only read an exact active immutable
//! generation. [`admit_source_evidence_use`] separately records which returned

//! evidence a consumer actually used.

mod coverage;
mod frontier;
mod lifecycle;
mod lineage;
mod model;
mod query;

pub use coverage::*;
pub use frontier::*;
pub use lifecycle::*;
pub use lineage::*;
pub use model::*;
pub use query::*;

use pale_kernel::AdaptiveNodeSetPolicy;

/// Construct separate lifecycle and query handles over one private registry.
///
/// Sharing storage does not widen authority: only the administrator can mutate
/// lifecycle state, and only the query service implements the query port.
#[must_use]
pub fn source_intelligence(
    set_policy: AdaptiveNodeSetPolicy,
) -> (SourceIndexAdministrator, SourceIntelligenceQueryService) {
    let administrator = SourceIndexAdministrator::new(set_policy);
    let queries = SourceIntelligenceQueryService::new(administrator.registry.clone());
    (administrator, queries)
}

#[cfg(test)]
#[path = "../../tests/internal/source_intelligence.rs"]
mod tests;
