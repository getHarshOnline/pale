//! One mutually authenticated exchange over a caller-selected local carrier.
//!
//! Both peers contribute fresh random challenges. Distinct signed proofs bind
//! their keys, challenges and exact request/response digests. The client sends
//! its private payload only after checking the pinned resident key. The server
//! releases a request only after verifying a selected caller's proof. This is
//! authentication, not encryption or Pale session/operation admission.

use std::time::Instant;

use ed25519_dalek::{Signature, VerifyingKey};
use pale_ports::{CryptographicRandomSourcePort, LocalConnectionSignerPort, LocalIpcFramePort};
use sha2::{Digest as _, Sha256};

mod client;
pub use client::{exchange_authenticated, AuthenticatedLocalClient};

const HELLO: &[u8; 16] = b"PaleAuthHelloV1\0";
const PROOF: &[u8; 16] = b"PaleAuthProofV1\0";
const REQUEST: &[u8; 16] = b"PaleAuthReqV1\0\0\0";
const RESPONSE: &[u8; 16] = b"PaleAuthRespV1\0\0";
const PROOF_DOMAIN: &[u8] = b"pale.local-connection.resident-proof.v1\0";
pub(crate) const REQUEST_DOMAIN: &[u8] = b"pale.local-connection.caller-request.v1\0";
const RESPONSE_DOMAIN: &[u8] = b"pale.local-connection.resident-response.v1\0";
const SIGNED_HEADER: usize = 16 + 64;

/// Maximum request or response bytes inside one authenticated exchange.
pub const LOCAL_AUTHENTICATED_PAYLOAD_BYTE_LIMIT: usize = 8 * 1024 * 1024;
/// Physical frame ceiling needed for the authentication header and payload.
pub const LOCAL_AUTHENTICATED_FRAME_BYTE_LIMIT: u32 =
    (LOCAL_AUTHENTICATED_PAYLOAD_BYTE_LIMIT + SIGNED_HEADER) as u32;

/// Why a local exchange could not prove its peer and exact bytes.
///
/// No variant contains keys, payloads or underlying diagnostics. On any error,
/// discard the connection and do not retry a possibly delivered request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LocalAuthenticationError {
    /// A physical carrier step failed; delivery may be incomplete or uncertain.
    #[error("authenticated local connection is unavailable")]
    Carrier,
    /// The original exchange deadline elapsed.
    #[error("authenticated local connection deadline elapsed")]
    Deadline,
    /// A frame has the wrong tag, shape, or payload bound.
    #[error("authenticated local connection frame is invalid")]
    Frame,
    /// A selected peer key or its proof was rejected.
    #[error("local connection peer proof was rejected")]
    Peer,
    /// Fresh operating-system randomness was unavailable.
    #[error("local connection challenge is unavailable")]
    Entropy,
    /// The selected connection credential could not sign.
    #[error("local connection signing is unavailable")]
    Signing,
}

/// Whether a failed client exchange could have delivered its application request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalRequestDelivery {
    /// Failure preceded the first attempt to send application bytes.
    NotSent,
    /// Sending began; the peer may have admitted and completed the request.
    Uncertain,
}

/// Safe client failure with its application-delivery uncertainty preserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{cause}")]
pub struct LocalAuthenticatedExchangeError {
    #[source]
    cause: LocalAuthenticationError,
    delivery: LocalRequestDelivery,
}

impl LocalAuthenticatedExchangeError {
    /// Identify the failed transport or authentication step without private bytes.
    #[must_use]
    pub const fn cause(self) -> LocalAuthenticationError {
        self.cause
    }

    /// Distinguish rejection before sending from a possibly delivered request.
    #[must_use]
    pub const fn request_delivery(self) -> LocalRequestDelivery {
        self.delivery
    }
}

struct Transcript {
    caller: [u8; 32],
    resident: [u8; 32],
    caller_nonce: [u8; 32],
    resident_nonce: [u8; 32],
}

impl Transcript {
    fn message(&self, domain: &[u8], suffix: &[u8]) -> Vec<u8> {
        let mut message = Vec::with_capacity(domain.len() + 128 + suffix.len());
        message.extend_from_slice(domain);
        message.extend_from_slice(&self.caller);
        message.extend_from_slice(&self.resident);
        message.extend_from_slice(&self.caller_nonce);
        message.extend_from_slice(&self.resident_nonce);
        message.extend_from_slice(suffix);
        message
    }
}

/// Exact bytes authenticated to one selected connection key.
///
/// This proves no principal, repository, provider, live session or permission.
/// The application must join the caller key to current authority and admit the
/// request before reading protected state or deciding whether work may proceed.
/// Consuming this value permits at most one response with the original deadline.
#[must_use = "an authenticated request must be admitted and answered or rejected"]
pub struct AuthenticatedLocalRequest {
    transcript: Transcript,
    payload: Box<[u8]>,
    request_digest: [u8; 32],
    deadline: Instant,
}

impl AuthenticatedLocalRequest {
    /// Borrow the selected caller key whose possession was proved.
    #[must_use]
    pub const fn caller_key(&self) -> &[u8; 32] {
        &self.transcript.caller
    }

    /// Borrow exact authenticated bytes, still awaiting application admission.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Bind one response to this request and deliver it before its deadline.
    ///
    /// # Errors
    /// Rejects a changed resident key, oversized response, elapsed deadline,
    /// signing failure or delivery failure. A delivery failure is uncertain;
    /// the application must not infer that its work did not happen.
    pub fn respond(
        self,
        carrier: &mut dyn LocalIpcFramePort,
        signer: &mut dyn LocalConnectionSignerPort,
        payload: &[u8],
    ) -> Result<(), LocalAuthenticationError> {
        check_deadline(self.deadline)?;
        check_payload(payload)?;
        if signer.public_key() != self.transcript.resident {
            return Err(LocalAuthenticationError::Peer);
        }
        let mut suffix = [0; 64];
        suffix[..32].copy_from_slice(&self.request_digest);
        suffix[32..].copy_from_slice(&Sha256::digest(payload));
        let signature = signer
            .sign(
                &self.transcript.message(RESPONSE_DOMAIN, &suffix),
                self.deadline,
            )
            .map_err(|_| LocalAuthenticationError::Signing)?;
        send_signed(carrier, RESPONSE, signature, payload, self.deadline)
    }
}

/// A resident proof sent to one selected, still-unproved caller.
///
/// Keep this value while waiting for the request frame. It owns no private
/// application input and cannot grant work. Its original deadline is fixed.
#[must_use = "a pending challenge must be completed or discarded"]
pub struct PendingLocalAuthentication {
    transcript: Transcript,
    deadline: Instant,
}

impl PendingLocalAuthentication {
    /// Verify one request after its carrier becomes readable.
    ///
    /// # Errors
    /// Rejects a changed or replayed proof, bad bounds, failed read or expired
    /// original deadline. No unverified payload is returned.
    pub fn receive_request(
        self,
        carrier: &mut dyn LocalIpcFramePort,
    ) -> Result<AuthenticatedLocalRequest, LocalAuthenticationError> {
        let Self {
            transcript,
            deadline,
        } = self;
        let frame = receive(carrier, deadline)?;
        let (signature, payload) = signed_parts(&frame, REQUEST)?;
        let request_digest: [u8; 32] = Sha256::digest(payload).into();
        verify(
            &transcript.caller,
            &transcript.message(REQUEST_DOMAIN, &request_digest),
            signature,
        )?;
        check_deadline(deadline)?;
        Ok(AuthenticatedLocalRequest {
            transcript,
            payload: payload.into(),
            request_digest,
            deadline,
        })
    }
}

/// Prove a selected caller with a blocking handshake and receive one request.
///
/// Event loops should use [`begin_authenticated_request`] and resume its
/// pending challenge after read readiness, so a quiet client does not occupy
/// the loop while application recovery is waiting.
///
/// # Errors
/// Preserves every failure from beginning and completing the exchange.
pub fn accept_authenticated_request(
    carrier: &mut dyn LocalIpcFramePort,
    signer: &mut dyn LocalConnectionSignerPort,
    random: &dyn CryptographicRandomSourcePort,
    deadline: Instant,
    select_caller: impl FnOnce(&[u8; 32]) -> bool,
) -> Result<AuthenticatedLocalRequest, LocalAuthenticationError> {
    begin_authenticated_request(carrier, signer, random, deadline, select_caller)?
        .receive_request(carrier)
}

/// Read one hello and send the resident proof without waiting for the request.
///
/// `select_caller` checks the claimed public key against deployment-selected
/// connection credentials. This selection is not yet proof of possession. It
/// must not treat an arbitrary key or matching OS user as permission.
/// No payload reaches the application before signature
/// verification. The caller still owns endpoint and kernel-peer policy.
///
/// # Errors
/// Rejects an unknown key, malformed hello, missing randomness, signing
/// failure, carrier failure or elapsed deadline.
pub fn begin_authenticated_request(
    carrier: &mut dyn LocalIpcFramePort,
    signer: &mut dyn LocalConnectionSignerPort,
    random: &dyn CryptographicRandomSourcePort,
    deadline: Instant,
    select_caller: impl FnOnce(&[u8; 32]) -> bool,
) -> Result<PendingLocalAuthentication, LocalAuthenticationError> {
    check_deadline(deadline)?;
    let hello = receive(carrier, deadline)?;
    if hello.len() != 80 || &hello[..16] != HELLO {
        return Err(LocalAuthenticationError::Frame);
    }
    let caller = array(&hello[16..48])?;
    if !select_caller(&caller) {
        return Err(LocalAuthenticationError::Peer);
    }
    let transcript = Transcript {
        caller,
        resident: signer.public_key(),
        caller_nonce: array(&hello[48..])?,
        resident_nonce: nonce(random)?,
    };
    let signature = signer
        .sign(&transcript.message(PROOF_DOMAIN, &[]), deadline)
        .map_err(|_| LocalAuthenticationError::Signing)?;
    let mut challenge = Vec::with_capacity(112);
    challenge.extend_from_slice(PROOF);
    challenge.extend_from_slice(&transcript.resident_nonce);
    challenge.extend_from_slice(&signature);
    send(carrier, &challenge, deadline)?;
    Ok(PendingLocalAuthentication {
        transcript,
        deadline,
    })
}

fn nonce(random: &dyn CryptographicRandomSourcePort) -> Result<[u8; 32], LocalAuthenticationError> {
    let mut bytes = [0; 32];
    random
        .fill_bytes(&mut bytes)
        .map_err(|_| LocalAuthenticationError::Entropy)?;
    Ok(bytes)
}

fn verify(
    key: &[u8; 32],
    message: &[u8],
    signature: [u8; 64],
) -> Result<(), LocalAuthenticationError> {
    VerifyingKey::from_bytes(key)
        .and_then(|key| key.verify_strict(message, &Signature::from_bytes(&signature)))
        .map_err(|_| LocalAuthenticationError::Peer)
}

fn array<const N: usize>(bytes: &[u8]) -> Result<[u8; N], LocalAuthenticationError> {
    bytes
        .try_into()
        .map_err(|_| LocalAuthenticationError::Frame)
}

fn check_deadline(deadline: Instant) -> Result<(), LocalAuthenticationError> {
    if Instant::now() >= deadline {
        Err(LocalAuthenticationError::Deadline)
    } else {
        Ok(())
    }
}

fn check_payload(payload: &[u8]) -> Result<(), LocalAuthenticationError> {
    if payload.is_empty() || payload.len() > LOCAL_AUTHENTICATED_PAYLOAD_BYTE_LIMIT {
        Err(LocalAuthenticationError::Frame)
    } else {
        Ok(())
    }
}

fn receive(
    carrier: &mut dyn LocalIpcFramePort,
    deadline: Instant,
) -> Result<Box<[u8]>, LocalAuthenticationError> {
    check_deadline(deadline)?;
    let bytes = carrier
        .receive_frame(deadline)
        .map_err(|_| LocalAuthenticationError::Carrier)?;
    if bytes.len() > LOCAL_AUTHENTICATED_FRAME_BYTE_LIMIT as usize {
        return Err(LocalAuthenticationError::Frame);
    }
    Ok(bytes)
}

fn send(
    carrier: &mut dyn LocalIpcFramePort,
    bytes: &[u8],
    deadline: Instant,
) -> Result<(), LocalAuthenticationError> {
    check_deadline(deadline)?;
    carrier
        .send_frame(bytes, deadline)
        .map_err(|_| LocalAuthenticationError::Carrier)
}

fn send_signed(
    carrier: &mut dyn LocalIpcFramePort,
    tag: &[u8; 16],
    signature: [u8; 64],
    payload: &[u8],
    deadline: Instant,
) -> Result<(), LocalAuthenticationError> {
    send(carrier, &signed_frame(tag, signature, payload), deadline)
}

fn signed_frame(tag: &[u8; 16], signature: [u8; 64], payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(SIGNED_HEADER + payload.len());
    frame.extend_from_slice(tag);
    frame.extend_from_slice(&signature);
    frame.extend_from_slice(payload);
    frame
}

fn signed_parts<'a>(
    frame: &'a [u8],
    tag: &[u8; 16],
) -> Result<([u8; 64], &'a [u8]), LocalAuthenticationError> {
    if frame.len() <= SIGNED_HEADER || &frame[..16] != tag {
        return Err(LocalAuthenticationError::Frame);
    }
    let payload = &frame[SIGNED_HEADER..];
    check_payload(payload)?;
    Ok((array(&frame[16..SIGNED_HEADER])?, payload))
}
