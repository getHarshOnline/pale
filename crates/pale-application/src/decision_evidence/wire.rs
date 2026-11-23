//! Strict bounded-view integrity; native authority and custody remain separate.

use pale_core::engine_wire::DecisionEvidenceLedgerV1;
use serde::{Deserialize, Serialize};

use super::current::DecisionEvidenceLedger;

impl Serialize for DecisionEvidenceLedger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        DecisionEvidenceLedgerV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            compilation_input_digest: self.compilation_input_digest.clone(),
            profile_digest: self.profile_digest.clone(),
            repository_id: self.repository_id.clone(),
            execution_path: self.execution_path.clone(),
            activity: self.activity.clone(),
            next_sequence: self.next_sequence,
            truncation_anchor: self.truncation_anchor.clone(),
            entries: self.entries.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DecisionEvidenceLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = DecisionEvidenceLedgerV1::deserialize(deserializer)?;
        let ledger = DecisionEvidenceLedger {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            compilation_input_digest: wire.compilation_input_digest,
            profile_digest: wire.profile_digest,
            repository_id: wire.repository_id,
            execution_path: wire.execution_path,
            activity: wire.activity,
            next_sequence: wire.next_sequence,
            truncation_anchor: wire.truncation_anchor,
            entries: wire.entries,
        };
        ledger.validate().map_err(serde::de::Error::custom)?;
        Ok(ledger)
    }
}
