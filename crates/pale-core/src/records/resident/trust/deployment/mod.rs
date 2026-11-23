//! Pure initial-v1 deployment trust contracts.
//!
//! The module forms an acyclic proof graph: [`DeploymentTermsV1`] feeds a
//! [`DeploymentBindingProposalV1`], an external semantic owner supplies a
//! [`DeploymentAcceptanceV1`], and the admitted pair closes into a
//! [`DeploymentBindingV1`]. A binding may then scope a
//! [`DeploymentRuntimeAuthorityV1`]. Acceptance revocation is an independent
//! monotone branch and cannot rewrite any predecessor record.
//!
//! The finalized [`DeploymentBindingV1`] owns the exact acyclic dependency
//! graph, monotone revocation branch, and protocol horizon.
//!
//! These types validate bytes, signatures, exact references, and contextual
//! proof obligations. They do not select workspace values, resolve external
//! authority by themselves, mutate a trust store, activate a runtime, or cause
//! a native effect.

mod acceptance;
mod binding;
mod contract_set;
mod genesis;
mod proposal;
mod runtime_authority;
mod terms;

pub use acceptance::*;
pub use binding::*;
pub use contract_set::*;
pub use genesis::*;
pub use proposal::*;
pub use runtime_authority::*;
pub use terms::*;

use super::*;
use crate::records::*;

#[cfg(test)]
#[path = "../../../../../tests/internal/records/resident/trust/deployment/mod.rs"]
mod tests;
