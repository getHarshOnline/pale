//! Package integration tests grouped by source loading, semantic catalogs and
//! current-source custody.

mod callers;
#[path = "../support/packages.rs"]
mod common;
mod frontier;
mod inspection;
