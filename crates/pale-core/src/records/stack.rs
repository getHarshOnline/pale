//! Platform and toolchain profiles used for doctrine projection.
//!
//! A [`Stack`] is a reusable platform identity with declarative semantic
//! relationships. Repository selection is explicit private configuration; a
//! package stack never selects itself from host files.

use serde::{Deserialize, Serialize};

use crate::records::{OfficialReference, PackageResourceIdV1, PackageResourceLinkV1};

/// One platform or toolchain profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    /// Stable stack identity used by selectors and overlays.
    pub id: PackageResourceIdV1,
    /// Compact platform or toolchain purpose.
    pub description: String,
    /// More-general stack identities required by this overlay. Projection closes
    /// this graph before selecting doctrine and official references.
    #[serde(default)]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Doctrine ids automatically relevant when this stack is selected.
    #[serde(default)]
    pub doctrine_refs: Vec<PackageResourceLinkV1>,
    /// Judgment-only doctrine safe to compose into read-only specialist roles for this stack.
    /// Every referenced entry is checked against its typed doctrine mechanics.
    #[serde(default)]
    pub role_doctrine_refs: Vec<PackageResourceLinkV1>,
    /// Exact official references for version-sensitive implementation claims.
    #[serde(default)]
    pub official_sources: Vec<OfficialReference>,
}

/// The distribution's stack catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StackCatalog {
    /// Version of the stack declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    #[serde(rename = "stack")]
    /// Stack declarations in authored catalog order.
    pub stacks: Vec<Stack>,
}

impl StackCatalog {
    /// A stack by id, if known.
    pub fn get(&self, id: &PackageResourceIdV1) -> Option<&Stack> {
        self.stacks.iter().find(|stack| &stack.id == id)
    }
}

/// `stacks.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StacksFile {
    /// The single stack catalog in this file.
    pub stacks: StackCatalog,
}
