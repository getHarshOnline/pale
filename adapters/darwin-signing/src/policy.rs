//! Closed native policy captured by the maintained filesystem owner.

use crate::{valid_coordinate, RecordKind, SigningBridgeRequest};
use pale_adapter_fs::{capture_private_configuration, PrivateConfigurationError, ReadLimit};
use pale_core::records::Ed25519PublicKeyV1;
use serde::Deserialize;
use std::{collections::HashSet, path::Path};

const POLICY_BYTES: usize = 16 * 1024;

/// Bounded by-value native policy capture; status zero admits exactly `length`
/// original JSON bytes. This contains native bindings, never secret seed data.
#[repr(C)]
pub struct SigningPolicyCapture {
    /// Zero success; 1 path input, 2 I/O, 3 protection, 4 bound/change, 5 shape.
    pub status: u32,
    /// Number of initialized original policy bytes on success.
    pub length: u32,
    /// First 4 KiB of the original captured bytes.
    pub block0: [u8; 4096],
    /// Second 4 KiB, with unused bytes zeroed.
    pub block1: [u8; 4096],
    /// Third 4 KiB, with unused bytes zeroed.
    pub block2: [u8; 4096],
    /// Fourth 4 KiB, with unused bytes zeroed.
    pub block3: [u8; 4096],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u8,
    client_requirement: String,
    client_uid: u32,
    maximum_connections: usize,
    slots: Vec<Slot>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Slot {
    alias: String,
    service: String,
    account: String,
    #[serde(default)]
    access_group: Option<String>,
    public_key: Ed25519PublicKeyV1,
    allowed_kinds: Vec<RecordKind>,
}

fn valid_policy(bytes: &[u8]) -> bool {
    let Ok(policy) = serde_json::from_slice::<Policy>(bytes) else {
        return false;
    };
    if policy.schema_version != 1
        || policy.client_requirement.is_empty()
        || policy.client_requirement.len() > 4096
        || policy.maximum_connections == 0
        || policy.maximum_connections > 64
        || policy.slots.is_empty()
        || policy.slots.len() > 64
    {
        return false;
    }
    // Syntax and actual code-identity acceptance are the native Security/XPC
    // owner's next boundary. A numeric UID is configuration, not a principal.
    let _configured_uid = policy.client_uid;
    let mut aliases = HashSet::new();
    for slot in policy.slots {
        if !valid_coordinate(&slot.alias)
            || slot.alias.len() > 128
            || !aliases.insert(slot.alias)
            || !valid_coordinate(&slot.service)
            || !valid_coordinate(&slot.account)
            || slot
                .access_group
                .as_ref()
                .is_some_and(|group| !valid_coordinate(group))
            || slot.allowed_kinds.is_empty()
            || slot.allowed_kinds.len() > 9
        {
            return false;
        }
        // The typed decoder already validates canonical base64url/length.
        let _expected_public_key = slot.public_key;
        let count = slot.allowed_kinds.len();
        if slot.allowed_kinds.into_iter().collect::<HashSet<_>>().len() != count {
            return false;
        }
    }
    true
}

/// Capture and validate an explicitly selected protected native policy file.
///
/// Input bytes are the exact UTF-8 absolute path, not a search hint or JSON.
/// The existing filesystem adapter owns no-follow traversal, effective-UID/
/// POSIX protection, regular single-link file and bounded capture. Rust owns
/// the closed policy shape; the returned bytes retain the original source.
/// Swift must also validate the native code requirement before starting XPC.
/// No key lookup, service start, grant or live configuration lease occurs.
///
/// As with execution, the exported ABI is fixed integer/byte structs by value;
/// no pointer or allocator ownership crosses the boundary.
#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn pale_darwin_signing_capture_policy_v1(
    input: SigningBridgeRequest,
) -> SigningPolicyCapture {
    let mut result = SigningPolicyCapture {
        status: 1,
        length: 0,
        block0: [0; 4096],
        block1: [0; 4096],
        block2: [0; 4096],
        block3: [0; 4096],
    };
    let Some(length) = usize::try_from(input.length)
        .ok()
        .filter(|n| *n <= input.bytes.len())
    else {
        return result;
    };
    let Ok(path) = std::str::from_utf8(&input.bytes[..length]) else {
        return result;
    };
    let bytes = match capture_private_configuration(Path::new(path), ReadLimit::new(POLICY_BYTES)) {
        Ok(bytes) => bytes,
        Err(error) => {
            result.status = match error {
                PrivateConfigurationError::Path => 1,
                PrivateConfigurationError::Io => 2,
                PrivateConfigurationError::Protection => 3,
                PrivateConfigurationError::Content => 4,
            };
            return result;
        }
    };
    if !valid_policy(&bytes) {
        result.status = 5;
        return result;
    }
    result.length = bytes.len() as u32;
    for (block, chunk) in [
        &mut result.block0,
        &mut result.block1,
        &mut result.block2,
        &mut result.block3,
    ]
    .into_iter()
    .zip(bytes.chunks(4096))
    {
        block[..chunk.len()].copy_from_slice(chunk);
    }
    result.status = 0;
    result
}

#[cfg(test)]
#[path = "../tests/internal/policy.rs"]
mod tests;
