//! Ed25519 signing core that runs inside Pale's privileged macOS XPC helper, so
//! the resident receives signatures without holding the key.
//!
//! This crate builds on `pale-core` and, on macOS, `pale-adapter-fs`. The Swift
//! helper in `native/` links it as a static library. The resident must receive
//! signatures through authenticated XPC, not load this key-reading library. Core
//! record owners still construct and admit preimages, canonical digests, signing
//! authority and transitions. This adapter creates no key, grant, trust record
//! or production helper binding.

#[cfg(all(feature = "qualification-fixture", not(debug_assertions)))]
compile_error!("the public signing fixture is forbidden in release builds");

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use pale_core::records::{
    Ed25519PublicKeyV1, A2A_TASK_BINDING_V1_SIGNATURE_DOMAIN,
    A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN, ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN,
    CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN,
    OPERATION_HANDLE_V1_SIGNATURE_DOMAIN, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN,
    PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN, SESSION_ADMISSION_LEASE_V1_SIGNATURE_DOMAIN,
};
use serde::Deserialize;
use zeroize::Zeroizing;

mod keychain;
#[cfg(target_os = "macos")]
mod policy;

#[cfg(target_os = "macos")]
pub use policy::{pale_darwin_signing_capture_policy_v1, SigningPolicyCapture};

const REQUEST_BYTES: usize = 4096;
static KEYCHAIN_ACCESS: Mutex<()> = Mutex::new(());

/// Identify the public fixture build before any qualification request is sent.
///
/// Returns one only for the debug fixture and zero for the normal Keychain core.
/// This read-only metadata is not signer identity or permission. It prevents
/// the native test from querying Keychain if given the wrong static library.
/// The export uses the same fixed, pointer-free native ABI as execution.
#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn pale_darwin_signing_is_fixture_v1() -> u32 {
    u32::from(cfg!(feature = "qualification-fixture"))
}

/// Fixed native ABI buffer. `length` identifies at most 4096 operation bytes.
///
/// All fields cross by value; there are no pointers, references, ownership
/// transfers or caller-provided Rust discriminants at the C boundary.
#[repr(C)]
pub struct SigningBridgeRequest {
    /// Number of initialized bytes at the start of `bytes`.
    pub length: u32,
    /// Bounded native request; contains lookup coordinates, never a seed.
    pub bytes: [u8; REQUEST_BYTES],
}

/// Fixed native ABI response; errors return a zeroed signature field.
#[repr(C)]
pub struct SigningBridgeResponse {
    /// Zero on success; otherwise one native refusal code documented below.
    pub status: u32,
    /// Raw Ed25519 signature only when `status` is zero.
    pub signature: [u8; 64],
}

#[derive(Clone, Copy, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RecordKind {
    SessionAdmissionLease,
    OperationHandle,
    OperationObservation,
    A2aTaskBinding,
    A2aTaskObservation,
    CapabilityResult,
    AdapterFailure,
    ControlStateCheckpoint,
    PrincipalTrustCheckpoint,
}

impl RecordKind {
    fn domain(self) -> &'static [u8] {
        match self {
            Self::SessionAdmissionLease => SESSION_ADMISSION_LEASE_V1_SIGNATURE_DOMAIN,
            Self::OperationHandle => OPERATION_HANDLE_V1_SIGNATURE_DOMAIN,
            Self::OperationObservation => OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN,
            Self::A2aTaskBinding => A2A_TASK_BINDING_V1_SIGNATURE_DOMAIN,
            Self::A2aTaskObservation => A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN,
            Self::CapabilityResult => CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN,
            Self::AdapterFailure => ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN,
            Self::ControlStateCheckpoint => CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN,
            Self::PrincipalTrustCheckpoint => PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u8,
    kind: RecordKind,
    digest: String,
    service: String,
    account: String,
    #[serde(default)]
    access_group: Option<String>,
    public_key: Ed25519PublicKeyV1,
    budget_ms: u32,
}

#[derive(Clone, Copy)]
#[repr(u32)]
enum Refusal {
    InvalidRequest = 1,
    Busy = 2,
    Deadline = 3,
    KeyUnavailable = 4,
    WrongKey = 5,
}

impl Request {
    fn validate(&self) -> Result<[u8; 32], Refusal> {
        if self.schema_version != 1
            || self.budget_ms == 0
            || self.budget_ms > 30_000
            || !valid_coordinate(&self.service)
            || !valid_coordinate(&self.account)
            || self
                .access_group
                .as_ref()
                .is_some_and(|s| !valid_coordinate(s))
            || self.digest.len() != 64
            || !self
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Refusal::InvalidRequest);
        }
        let mut digest = [0; 32];
        for (out, pair) in digest
            .iter_mut()
            .zip(self.digest.as_bytes().chunks_exact(2))
        {
            let digit = |b: u8| {
                if b.is_ascii_digit() {
                    b - b'0'
                } else {
                    b - b'a' + 10
                }
            };
            *out = digit(pair[0]) * 16 + digit(pair[1]);
        }
        Ok(digest)
    }
}

fn valid_coordinate(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn sign_with(
    request: &Request,
    access: &Mutex<()>,
    deadline: Instant,
    lookup: impl FnOnce(&Request) -> Result<Zeroizing<Vec<u8>>, Refusal>,
) -> Result<[u8; 64], Refusal> {
    let digest = request.validate()?;
    let _guard = access.try_lock().map_err(|_| Refusal::Busy)?;
    if Instant::now() >= deadline {
        return Err(Refusal::Deadline);
    }
    let seed = lookup(request)?;
    if Instant::now() >= deadline {
        return Err(Refusal::Deadline);
    }
    let seed_bytes: &[u8; 32] = seed
        .as_slice()
        .try_into()
        .map_err(|_| Refusal::KeyUnavailable)?;
    let key = SigningKey::from_bytes(seed_bytes);
    if URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes()) != request.public_key.as_str() {
        return Err(Refusal::WrongKey);
    }
    let mut message = Vec::with_capacity(request.kind.domain().len() + digest.len());
    message.extend_from_slice(request.kind.domain());
    message.extend_from_slice(&digest);
    let signature = key.sign(&message).to_bytes();
    if Instant::now() >= deadline {
        return Err(Refusal::Deadline);
    }
    Ok(signature)
}

/// Execute one bounded native-helper signing request.
///
/// This function belongs inside the authenticated helper; it performs no caller
/// authentication itself. Its XPC owner must first select an allowlisted slot,
/// record kind and expected public key from protected configuration. Request
/// JSON is closed and versioned. The signature covers the core-owned domain
/// (including its NUL) followed by the decoded 32-byte SHA-256 digest, using
/// ordinary Ed25519, not Ed25519ph. No arbitrary domain or key bytes are accepted.
///
/// Status codes: 1 invalid input; 2 occupied/poisoned serialization guard;
/// 3 deadline; 4 unavailable/ambiguous/invalid native key; 5 unlike public key.
/// The caller's XPC wait needs its own absolute deadline: the OS Keychain call
/// cannot be interrupted here. Expired work returns no usable signature.
///
/// The one unsafe attribute exports a fixed, project-prefixed C symbol. The ABI
/// consists only of `repr(C)` integer/byte structs passed by value. No unsafe
/// Rust block, dereference, allocation ownership or pointer lifetime crosses it.
#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn pale_darwin_signing_execute_v1(
    input: SigningBridgeRequest,
) -> SigningBridgeResponse {
    let started = Instant::now();
    let result = usize::try_from(input.length)
        .ok()
        .filter(|len| *len <= REQUEST_BYTES)
        .and_then(|len| serde_json::from_slice::<Request>(&input.bytes[..len]).ok())
        .ok_or(Refusal::InvalidRequest)
        .and_then(|request| {
            let deadline = started + Duration::from_millis(u64::from(request.budget_ms));
            sign_with(&request, &KEYCHAIN_ACCESS, deadline, keychain::read_seed)
        });
    match result {
        Ok(signature) => SigningBridgeResponse {
            status: 0,
            signature,
        },
        Err(reason) => SigningBridgeResponse {
            status: reason as u32,
            signature: [0; 64],
        },
    }
}

#[cfg(test)]
#[path = "../tests/internal/signing.rs"]
mod tests;
