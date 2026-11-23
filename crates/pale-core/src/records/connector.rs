//! Neutral connector identities understood by the distribution.
//!
//! A connector contract says which external capability family Pale can bind
//! and the effect classes it may carry. Concrete servers, credentials, and host
//! tool names remain private project or machine configuration.

use serde::{Deserialize, Serialize};

use crate::records::EffectClass;

/// Distribution-owned contract for one provider-neutral connector family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorContract {
    /// Stable provider-neutral connector identity.
    pub id: String,
    /// Compact purpose shown when configuring a provider binding.
    pub description: String,
    /// Maximum effect classes a private binding may expose through this connector.
    pub effect_classes: Vec<EffectClass>,
}

/// Connector contracts available for project-local provider binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorCatalog {
    /// Version of the connector declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    #[serde(rename = "connector")]
    /// Connector contracts in authored catalog order.
    pub connectors: Vec<ConnectorContract>,
}

impl ConnectorCatalog {
    /// A connector contract by id, if known.
    pub fn get(&self, id: &str) -> Option<&ConnectorContract> {
        self.connectors.iter().find(|connector| connector.id == id)
    }
}

/// Root record of the distribution connector catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorFile {
    /// The single connector catalog in this file.
    pub connectors: ConnectorCatalog,
}
