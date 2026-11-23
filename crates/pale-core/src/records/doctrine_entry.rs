//! Model-facing doctrine with typed composition boundaries.
//!
//! A [`DoctrineEntry`] is a discoverable Markdown body whose substance remains
//! model-facing. Placement distinguishes on-demand judgment, operation-owned
//! procedure, and startup policy. Rust enforces that delivery boundary without
//! interpreting the prose.

use serde::{Deserialize, Serialize};

use crate::paths::OwnedConfinedRelativePath;
use crate::records::{PackageResourceIdV1, PackageResourceLinkV1};

/// HTTPS location of one official, version-sensitive source.
///
/// The type validates only the transport floor Pale can prove without
/// resolving the address: an explicit `https://` prefix, at least one following
/// character, and no whitespace. Publisher authority and whether the source is
/// genuinely official remain semantic-admission concerns.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(try_from = "String", into = "String")]
#[schemars(extend("pattern" = r"^https://[^\s]+$"))]
pub struct OfficialSourceUrl(String);

impl OfficialSourceUrl {
    /// Parse the structural HTTPS source-location contract.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidOfficialSourceUrl`] unless `value` begins with
    /// `https://`, has a non-empty remainder, and contains no whitespace.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidOfficialSourceUrl> {
        let value = value.into();
        let suffix = value.strip_prefix("https://");
        if suffix.is_none_or(str::is_empty) || value.chars().any(char::is_whitespace) {
            return Err(InvalidOfficialSourceUrl);
        }
        Ok(Self(value))
    }

    /// Borrow the validated HTTPS spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OfficialSourceUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for OfficialSourceUrl {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<String> for OfficialSourceUrl {
    type Error = InvalidOfficialSourceUrl;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<OfficialSourceUrl> for String {
    fn from(value: OfficialSourceUrl) -> Self {
        value.0
    }
}

/// An official-source location violates the portable HTTPS wire contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("official source URL must use HTTPS, contain a location, and contain no whitespace")]
pub struct InvalidOfficialSourceUrl;

/// Where one doctrine body may enter a native harness context.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DoctrinePlacement {
    /// Discoverable on demand and safe to compose into a read-only specialist.
    StandaloneJudgment,
    /// Nuanced procedure composed only inside an admitted operation.
    OperationProcedure,
    /// Compact policy delivered through the sealed startup context, never as a skill.
    StartupPolicy,
}

impl std::fmt::Display for DoctrinePlacement {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::StandaloneJudgment => "standalone_judgment",
            Self::OperationProcedure => "operation_procedure",
            Self::StartupPolicy => "startup_policy",
        })
    }
}

/// A version-sensitive primary source the model may consult when doctrine alone
/// is insufficient. URLs must point to official project/vendor documentation;
/// the compiler presents at most two relevant references per composed task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OfficialReference {
    /// Compact source label shown beside the doctrine.
    pub label: String,
    /// Exact official source URL.
    pub url: OfficialSourceUrl,
    /// Optional jurisdiction for legal, regulatory, or standards-bound material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<String>,
    /// Date on which this project last reviewed the reference (YYYY-MM-DD).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<String>,
}

/// One model-facing doctrine entry. It carries no tool authority; placement only
/// constrains where its prose may be composed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DoctrineEntry {
    /// Stable doctrine identity used by composition references.
    pub id: PackageResourceIdV1,
    /// Compact purpose shown in generated indexes.
    pub description: String,
    /// Relative path (under the package) to the Markdown doctrine body carrying the knowledge.
    pub doctrine: OwnedConfinedRelativePath,
    /// Exact context surface allowed to receive this body's prose.
    pub placement: DoctrinePlacement,
    /// Other doctrine ids composed before this entry.
    #[serde(default)]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Independent doctrine that is required to apply this entry correctly.
    /// Required bodies are composed into the rendered skill without pretending
    /// that this entry inherits their identity.
    #[serde(default)]
    pub requires: Vec<PackageResourceLinkV1>,
    /// Related doctrine that remains optional and progressively disclosed.
    /// The compiler materializes these entries and the renderer exposes exact
    /// skill names, but does not inject their bodies automatically.
    #[serde(default)]
    pub see_also: Vec<PackageResourceLinkV1>,
    /// Exact primary sources for uncertain or version-sensitive claims.
    #[serde(default)]
    pub official_sources: Vec<OfficialReference>,
}

/// The distribution's model-judgment doctrine catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DoctrineCatalog {
    /// Version of the doctrine declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    #[serde(rename = "entry")]
    /// Doctrine declarations in authored catalog order.
    pub entries: Vec<DoctrineEntry>,
}

impl DoctrineCatalog {
    /// A doctrine entry by id, if known.
    pub fn get(&self, id: &PackageResourceIdV1) -> Option<&DoctrineEntry> {
        self.entries.iter().find(|entry| &entry.id == id)
    }
}

/// `doctrine.toml` (one per family under the doctrine home).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DoctrineFile {
    /// The single doctrine catalog in this file.
    pub doctrine: DoctrineCatalog,
}

#[cfg(test)]
#[path = "../../tests/internal/records/doctrine_entry.rs"]
mod tests;
