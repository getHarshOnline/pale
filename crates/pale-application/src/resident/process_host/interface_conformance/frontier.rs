//! Real in-memory Start, Advance, and Observe conformance frontier.

use super::{
    evidence::{captured_generation, lifecycle, principal, request, session},
    memory::{anchor, custody, storage, writer},
    OperationConformanceFrontierError,
};

pub(super) mod application;

pub(super) mod owners;

pub(super) use application::*;
pub(super) use owners::*;
