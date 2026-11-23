//! Compile workspace configuration, admit tool calls, and manage resident state.
//!
//! For configuration work, [`workspace`] resolves declarations and [`compile`]
//! retains their input snapshot. [`semantic`] derives repository-specific
//! meaning from admitted Packages; [`render`] produces the repository content.
//! [`plan`] constructs and checks materialization plans without changing a
//! target tree.
//!
//! For agent tool calls, [`hooks`] evaluates declared lifecycle policy and
//! encodes the selected harness response. [`operation_admission`] checks the
//! selected operation, capability bindings, and authorization evidence. Neither
//! a hook response nor a plan proves that a native action succeeded.
//!
//! [`resident`] commits resident-state transitions and reconstructs them after
//! restart. It derives process instructions from desired state and separately
//! supplied observations; it does not start processes. [`observation`] exposes
//! compiler and resident facts without turning desired state into observed
//! process health.
//!
//! This crate builds on `pale-core`, `pale-contracts`, `pale-kernel` and
//! [`pale_ports`]; `pale-sdk` and `pale-resident` build on it. Callers supply
//! filesystem, process, and storage implementations through the ports. This
//! crate owns the checks and transitions, not the selection of concrete
//! adapters. The SDK composes local adapters behind its native operations so
//! interfaces need not rebuild that work.
//!
//! A [`compile::Compilation`] is a one-shot content identity. It is not a
//! resident generation, activation, owner receipt, or proof of native effect.

pub mod actions;
mod bounded_text;
mod captured_input;
pub mod compile;
mod composition;
pub mod decision_evidence;
pub mod diagnostic;
pub mod doctor;
mod failure;
pub mod hooks;
mod host_command;
mod host_tool;
mod identity;
pub mod improvement;
mod lifecycle;
mod markdown;
pub mod observation;
pub mod operation_admission;
pub mod packages;
pub mod plan;
pub mod provider_invocation;
pub mod render;
pub mod resident;
pub mod semantic;
pub mod service_topology;
pub mod session_state;
pub mod source_intelligence;
pub mod terminal_adapters;
pub mod workspace;

pub use failure::{embedded_schema_failure_disposition, ApplicationFailureDisposition};

#[cfg(test)]
#[path = "../tests/support/resident.rs"]
mod resident_fixture;

#[cfg(test)]
#[path = "../tests/support/effect_case_lineage.rs"]
mod effect_case_lineage_fixture;

#[cfg(test)]
#[path = "../tests/support/operation_accumulator.rs"]
mod operation_accumulator_fixture;

#[cfg(test)]
#[path = "../tests/support/composition_source.rs"]
mod composition_source_fixtures;
#[cfg(test)]
#[path = "../tests/support/organization_composition.rs"]
mod organization_composition_fixtures;
#[cfg(test)]
#[path = "../tests/support/organization.rs"]
mod organization_fixtures;
#[cfg(test)]
#[path = "../tests/support/packages.rs"]
mod package_fixtures;
#[cfg(test)]
#[path = "../tests/support/provider_binding.rs"]
mod provider_binding_fixtures;
#[cfg(test)]
#[path = "../tests/support/responsibility.rs"]
mod responsibility_fixtures;

#[cfg(test)]
#[path = "../tests/support/principal_trust_bootstrap.rs"]
mod principal_trust_test_support;

#[cfg(test)]
#[path = "../tests/support/deployment_finalization.rs"]
mod deployment_finalization_fixture;

#[cfg(test)]
#[path = "../tests/support/producer_records.rs"]
mod producer_record_fixture;

#[cfg(test)]
#[path = "../tests/support/producer_custody.rs"]
mod producer_custody_fixture;

#[cfg(test)]
#[path = "../tests/support/session_admission.rs"]
mod session_admission_fixture;

#[cfg(test)]
#[path = "../tests/support/local_operation_start.rs"]
mod local_operation_start_fixture;
