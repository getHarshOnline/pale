//! Exact-key v3 JSON gateway reads with bounded bodies and verified cluster ID.

use std::io::Read;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use pale_ports::{AntiRollbackAnchor, AntiRollbackAnchorKey};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{codec, EtcdAnchorStore, StoreFailure};

pub(super) const RESPONSE_BYTES: u64 = 64 * 1024;

pub(super) struct Observed {
    pub(super) anchor: AntiRollbackAnchor,
    pub(super) bytes: Vec<u8>,
    pub(super) revision: String,
}

#[derive(Deserialize)]
pub(super) struct Header {
    // Native transaction subresponse headers carry the revision but omit IDs.
    // The root response must still pass verify_header before any result is used.
    #[serde(default)]
    cluster_id: String,
    #[serde(default)]
    member_id: String,
    pub(super) revision: String,
}

#[derive(Deserialize)]
pub(super) struct Range {
    pub(super) header: Option<Header>,
    #[serde(default)]
    kvs: Vec<KeyValue>,
    #[serde(default)]
    count: String,
    #[serde(default)]
    more: bool,
}

#[derive(Deserialize)]
struct KeyValue {
    key: String,
    value: String,
    mod_revision: String,
    #[serde(default)]
    lease: String,
}

impl EtcdAnchorStore {
    pub(super) fn post(&self, route: &str, request: Value) -> Result<Value, StoreFailure> {
        let bytes = serde_json::to_vec(&request).map_err(|_| StoreFailure::Protocol)?;
        if bytes.len() as u64 > RESPONSE_BYTES {
            return Err(StoreFailure::Protocol);
        }
        let mut url = self.endpoint.clone();
        url.set_path(route);
        let response = self
            .client
            .post(url)
            .header(reqwest::header::AUTHORIZATION, self.token.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(bytes)
            .send()
            .map_err(|_| StoreFailure::Transport)?;
        if !response.status().is_success() {
            return Err(StoreFailure::Status(response.status().as_u16()));
        }
        let mut bytes = Vec::new();
        response
            .take(RESPONSE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| StoreFailure::Transport)?;
        if bytes.len() as u64 > RESPONSE_BYTES {
            return Err(StoreFailure::Protocol);
        }
        serde_json::from_slice(&bytes).map_err(|_| StoreFailure::Protocol)
    }

    pub(super) fn verify_header(&self, header: &Header) -> Result<(), StoreFailure> {
        if header.cluster_id.parse::<u64>().ok() != Some(self.cluster_id.get())
            || header.member_id.parse::<u64>().ok().is_none_or(|v| v == 0)
            || header.revision.parse::<u64>().ok().is_none_or(|v| v == 0)
        {
            return Err(StoreFailure::Protocol);
        }
        Ok(())
    }

    pub(super) fn range(
        &self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<Observed>, StoreFailure> {
        let raw_key = codec::physical_key(&self.prefix, key)?;
        let response = self.post("/v3/kv/range", range_request(&raw_key))?;
        let range: Range = serde_json::from_value(response).map_err(|_| StoreFailure::Protocol)?;
        self.verify_header(range.header.as_ref().ok_or(StoreFailure::Protocol)?)?;
        decode_range(range, &raw_key, key)
    }
}

pub(super) fn range_request(raw_key: &[u8]) -> Value {
    json!({"key":STANDARD.encode(raw_key), "limit":"2", "serializable":false})
}

pub(super) fn decode_range(
    range: Range,
    raw_key: &[u8],
    key: &AntiRollbackAnchorKey,
) -> Result<Option<Observed>, StoreFailure> {
    if range.more {
        return Err(StoreFailure::Protocol);
    }
    if range.kvs.is_empty() && matches!(range.count.as_str(), "" | "0") {
        return Ok(None);
    }
    if range.kvs.len() != 1 || range.count != "1" {
        return Err(StoreFailure::Protocol);
    }
    let entry = range.kvs.into_iter().next().ok_or(StoreFailure::Protocol)?;
    if STANDARD
        .decode(entry.key)
        .map_err(|_| StoreFailure::Protocol)?
        != raw_key
        || entry
            .mod_revision
            .parse::<u64>()
            .ok()
            .is_none_or(|v| v == 0)
        || !matches!(entry.lease.as_str(), "" | "0")
    {
        return Err(StoreFailure::Protocol);
    }
    let bytes = STANDARD
        .decode(entry.value)
        .map_err(|_| StoreFailure::Protocol)?;
    let anchor = codec::decode(&bytes)?;
    if &anchor.key() != key {
        return Err(StoreFailure::Protocol);
    }
    Ok(Some(Observed {
        anchor,
        bytes,
        revision: entry.mod_revision,
    }))
}
