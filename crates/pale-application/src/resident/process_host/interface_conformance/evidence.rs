//! Admitted source and authority evidence for interface conformance.

pub(super) mod captured_generation;

pub(super) mod principal;

pub(super) mod session;

pub(super) mod lifecycle;

pub(super) mod request;

pub(super) use captured_generation::*;
pub(super) use request::*;

#[cfg(test)]
#[path = "../../../../tests/internal/resident/process_host/interface_test_support/captured_generation.rs"]
mod captured_generation_tests;

#[cfg(test)]
#[path = "../../../../tests/internal/resident/process_host/interface_test_support/principal.rs"]
mod principal_tests;

#[cfg(test)]
#[path = "../../../../tests/support/interface_conformance_request.rs"]
pub(crate) mod request_test_support;
