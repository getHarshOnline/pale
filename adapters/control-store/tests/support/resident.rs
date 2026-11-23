//! Shared admitted-generation fixtures for real `SQLite` recovery tests.

use pale_application::{compile, hooks, packages, plan, resident, workspace};

#[cfg(test)]
use pale_application::{actions, semantic};

#[path = "../../../../crates/pale-application/tests/support/composition_source.rs"]
mod composition_source_fixtures;
#[path = "../../../../crates/pale-application/tests/support/packages.rs"]
mod package_fixtures;
#[path = "../../../../crates/pale-application/tests/support/provider_binding.rs"]
mod provider_binding_fixtures;
#[path = "../../../../crates/pale-application/tests/support/resident.rs"]
pub(crate) mod shared_resident;

// The session fixture is shared with `pale-application`, where these helpers
// live directly under `crate::resident_fixture`. Preserve that same local
// fixture path when the source is compiled inside this adapter.
#[cfg(test)]
pub(crate) use shared_resident::{active, advertised, fence, published};

#[cfg(test)]
use shared_resident as resident_fixture;
#[cfg(test)]
#[path = "../../../../crates/pale-application/tests/support/deployment_finalization.rs"]
pub(crate) mod deployment_finalization_fixture;
#[cfg(test)]
#[path = "../../../../crates/pale-application/tests/support/effect_case_lineage/request.rs"]
pub(crate) mod effect_case_lineage_fixture;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../../../crates/pale-application/tests/support/local_operation_start.rs"]
pub(crate) mod local_operation_start_fixture;
#[cfg(test)]
#[path = "../../../../crates/pale-application/tests/support/principal_checkpoint.rs"]
pub(crate) mod principal_checkpoint_fixture;
#[cfg(test)]
#[path = "../../../../crates/pale-application/tests/support/principal_trust_bootstrap.rs"]
mod principal_trust_test_support;
#[cfg(test)]
#[path = "../../../../crates/pale-application/tests/support/producer_custody.rs"]
pub(crate) mod producer_custody_fixture;
#[cfg(test)]
#[path = "../../../../crates/pale-application/tests/support/producer_records.rs"]
mod producer_record_fixture;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../../../crates/pale-application/tests/support/session_admission.rs"]
pub(crate) mod session_admission_fixture;
