//! Bounded native and connector-backed capabilities.
//!
//! A [`BoundedTool`] declares a prospective executable capability. [`crate::records::DoctrineEntry`]
//! owns model judgment, while [`crate::records::OnDemandOperation`] owns multi-step
//! procedures. Distribution support, private provider binding, and operation
//! admission remain separate constraints. Presence in this wire catalog proves
//! neither implementation, provider availability, nor an effect.

use serde::{Deserialize, Serialize};

use crate::records::{Approval, Availability, EffectClass, Mechanics};

/// One host-neutral capability with an explicit mechanic, effect, approval, and availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoundedTool {
    /// Stable host-neutral tool identity.
    pub id: String,
    /// Compact purpose shown during tool discovery.
    pub description: String,
    /// How the effect is carried out.
    pub mechanics: Mechanics,
    /// Declared compiled action ID backing an available native-Rust tool.
    /// Required for `native_rust + available_now`; absent for declarations whose
    /// mechanic has not been implemented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_action: Option<String>,
    /// Declared connector ID. Semantic admission resolves it for connector
    /// mechanics; it is absent for
    /// every other mechanic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    /// The neutral effect class of invoking the tool (read / execute / write / network / publish).
    pub effect_class: EffectClass,
    /// Whether invoking the tool needs confirmation or a bounded authorization.
    #[serde(default = "default_approval")]
    pub approval: Approval,
    /// Declared distribution support. `available_now` still requires catalog
    /// admission, an implemented action or bindable connector, private provider
    /// selection where applicable, and an admitted operation before use.
    #[serde(default = "default_availability")]
    pub availability: Availability,
    /// Named owner of the unavailable durable effect when `availability = governed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governed_by: Option<String>,
    /// Package-relative Markdown guidance for judgment around invocation and
    /// inputs. This prose grants no mechanical authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doctrine: Option<String>,
    /// Tool family used to group the catalog without another record type.
    pub family: String,
}

fn default_approval() -> Approval {
    Approval::Ask
}

/// Honest default: a declared mechanic is not assumed implemented or bindable.
fn default_availability() -> Availability {
    Availability::HonestUnavailable
}

/// The distribution's bounded-tool catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolCatalog {
    /// Version of the bounded-tool declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    #[serde(rename = "tool")]
    /// Tool declarations in authored catalog order.
    pub tools: Vec<BoundedTool>,
}

impl ToolCatalog {
    /// The first tool declaration with `id`, if any.
    ///
    /// Semantic admission must establish ID uniqueness before this lookup is used.
    pub fn get(&self, id: &str) -> Option<&BoundedTool> {
        self.tools.iter().find(|t| t.id == id)
    }

    /// Tool declarations marked [`Availability::AvailableNow`].
    ///
    /// This is a field projection, not proof that a mechanic is implemented or
    /// bindable. Project availability still requires semantic and private binding
    /// admission.
    pub fn available_now(&self) -> impl Iterator<Item = &BoundedTool> {
        self.tools
            .iter()
            .filter(|t| t.availability == Availability::AvailableNow)
    }
}

/// `tools.toml` (one per family under the tools home).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolFile {
    /// The single bounded-tool catalog in this file.
    pub tool: ToolCatalog,
}
