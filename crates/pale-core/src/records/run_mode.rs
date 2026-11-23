//! User-selectable session modes and their doctrine payloads.
//!
//! A [`RunMode`] carries model-facing guidance for one independent mode axis. The
//! active choice is mutable session state; the allowed modes and payload digests
//! are sealed in the hook context. Modes tune guidance and do not grant tool
//! authority. Runtime and session scope are always authored explicitly.
//! Raw catalogs remain declarations until the package loader validates IDs,
//! axes, aliases, extension relationships, defaults, and doctrine paths.

use serde::{Deserialize, Serialize};

use crate::paths::OwnedConfinedRelativePath;
use crate::records::{ModeAxis, ModeScope, PackageResourceIdV1, PackageResourceLinkV1};

/// One selectable value on a session-mode axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunMode {
    /// Stable mode identity within its axis.
    pub id: PackageResourceIdV1,
    /// Compact purpose shown in mode discovery and activation output.
    pub description: String,
    /// The independent session dimension this mode occupies.
    pub axis: ModeAxis,
    /// Package-relative Markdown guidance. A suppressing mode omits the body
    /// instead of pointing to an empty artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doctrine: Option<OwnedConfinedRelativePath>,
    /// Prompt aliases that switch to this mode (e.g. `/lite`).
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Standalone natural-language messages that switch to this mode. These are
    /// exact whole-message triggers, never substring matches.
    #[serde(default)]
    pub exact_triggers: Vec<String>,
    /// Same-axis mode guidance composed before this mode's own body.
    #[serde(default)]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Whether the mode can be a persisted default (`runtime`) or is activated per-session only.
    pub scope: ModeScope,
}

/// The distribution's run-mode catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunModeCatalog {
    /// Version of the run-mode declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    #[serde(rename = "mode")]
    /// Mode declarations in authored catalog order.
    pub modes: Vec<RunMode>,
}

impl RunModeCatalog {
    /// The first mode declaration with `id`, if any.
    ///
    /// Semantic admission must establish ID uniqueness before this lookup is used.
    pub fn get(&self, id: &PackageResourceIdV1) -> Option<&RunMode> {
        self.modes.iter().find(|mode| &mode.id == id)
    }

    /// Whether this catalog declares `id` as a runtime-scoped mode on `axis`.
    ///
    /// This query does not validate the rest of the catalog or select a default.
    #[must_use]
    pub fn is_runtime_mode(&self, id: &PackageResourceIdV1, axis: ModeAxis) -> bool {
        self.get(id)
            .is_some_and(|mode| mode.scope == ModeScope::Runtime && mode.axis == axis)
    }
}

/// `run-modes.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunModesFile {
    /// The single run-mode catalog in this file.
    pub run_modes: RunModeCatalog,
}
