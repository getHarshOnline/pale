//! Producer retries preserve bytes; fixture request authority remains modeled.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ed25519_dalek::{Signer, SigningKey};
use pale_agent_host::LocalAgentNativeToolPreparation;
use pale_sdk::{
    AuthenticatedResidentCarrier, AuthenticatedResidentCarrierFailure,
    AuthenticatedResidentCarrierSource, AuthenticatedResidentCarrierSourceFailure,
    DiagnosticEffectCertainty, PaleApi, ResidentAttachment, ResidentHookRequest,
    ResidentNativeRequestSigner, ResidentNativeRequestSigningFailure,
    ResidentNativeRequestSigningInput, ResidentResponseAdmission, ResidentResponseAdmissionFailure,
};

fn preparation(api: &PaleApi) -> LocalAgentNativeToolPreparation {
    LocalAgentNativeToolPreparation::new(api, ResidentHookRequest::new(
        "pre-tool-use".into(), "workspace-1".into(), "repository-1".into(),
        "terminal".into(), "a".repeat(64),
        br#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_use_id":"tool-1","tool_name":"Read","tool_input":{"file_path":"source.rs"}}"#.to_vec(),
    )).unwrap()
}

fn unsigned_request(body: &[u8]) -> Vec<u8> {
    // Reuse the application's complete public request fixture rather than
    // maintaining another authority-record model in this integration.
    let signed = pale_application::resident::with_in_memory_invoke_conformance_frontier(
        |_, requests| {
            (
                pale_ports::ResidentOperationFrameLocator::new(
                    "fixture-handle".into(),
                    "a".repeat(64),
                ),
                requests.start_signed_common_request().to_owned(),
            )
        },
        |_, _, _, signed| signed,
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&signed).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("canonical_request_sha256");
    object.remove("caller_signature");
    value["body"] = serde_json::from_slice(body).unwrap();
    serde_json::to_vec(&value).unwrap()
}

struct RequestSigner {
    calls: usize,
    expected: Vec<u8>,
    key: SigningKey,
    message: Vec<u8>,
    signature: Option<ed25519_dalek::Signature>,
}

impl ResidentNativeRequestSigner for RequestSigner {
    fn sign(
        &mut self,
        input: ResidentNativeRequestSigningInput<'_>,
        deadline: Instant,
    ) -> Result<String, ResidentNativeRequestSigningFailure> {
        self.calls += 1;
        assert!(Instant::now() < deadline);
        assert_eq!(input.canonical_preimage(), self.expected);
        assert_eq!(input.purpose(), b"PALE_COMMON_REQUEST_ENVELOPE_V1\0");
        // This fixture authorizes exactly the preselected claim and purpose.
        self.message = input.purpose().to_vec();
        self.message.extend(
            (0..64)
                .step_by(2)
                .map(|i| u8::from_str_radix(&input.digest()[i..i + 2], 16).unwrap()),
        );
        let signature = self.key.sign(&self.message);
        self.signature = Some(signature);
        Ok(base64url(&signature.to_bytes()))
    }
}

// Test-only encoding of the fixed 64-byte signature; no production key handling.
fn base64url(bytes: &[u8; 64]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::new();
    let mut accumulator = 0u32;
    let mut bits = 0;
    for byte in bytes {
        accumulator = (accumulator << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            encoded.push(char::from(ALPHABET[((accumulator >> bits) & 63) as usize]));
        }
    }
    if bits != 0 {
        encoded.push(char::from(
            ALPHABET[((accumulator << (6 - bits)) & 63) as usize],
        ));
    }
    encoded
}

struct LostResponses(Arc<Mutex<Vec<Vec<u8>>>>);
impl AuthenticatedResidentCarrierSource for LostResponses {
    fn open(
        &mut self,
        _: Instant,
    ) -> Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>
    {
        Ok(Box::new(Self(Arc::clone(&self.0))))
    }
}
impl AuthenticatedResidentCarrier for LostResponses {
    fn exchange(
        &mut self,
        request: &[u8],
        _: Instant,
    ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
        self.0.lock().unwrap().push(request.to_vec());
        Err(AuthenticatedResidentCarrierFailure::OutcomeUnknown)
    }
}
struct NoResponses;
impl ResidentResponseAdmission for NoResponses {
    fn admit(
        &self,
        _: &str,
        _: &[u8],
        _: &str,
        _: &[u8],
        _: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        panic!("no signed response was received")
    }
}

#[test]
fn explicit_start_retry_preserves_the_request_and_never_signs_twice() {
    let api = PaleApi::new().unwrap();
    let preparation = preparation(&api);
    let unsigned = unsigned_request(preparation.request_body_json());
    let mut signer = RequestSigner {
        calls: 0,
        expected: unsigned.clone(),
        key: SigningKey::from_bytes(&[31; 32]),
        message: Vec::new(),
        signature: None,
    };
    let mut pending = preparation
        .sign(
            &api,
            &unsigned,
            &mut signer,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        Box::new(LostResponses(Arc::clone(&sent))),
        Duration::from_secs(2),
        Arc::new(NoResponses),
    )
    .unwrap();
    for count in 1..=2 {
        let error = pending.start(&api, Some(&attachment)).unwrap_err();
        assert_eq!(
            error.diagnostic().effect(),
            DiagnosticEffectCertainty::OutcomeUnknown
        );
        assert_eq!(sent.lock().unwrap().len(), count);
        assert_eq!(signer.calls, 1);
        assert!(pending.observation().is_none());
        assert!(pending.deliver_hook(&api, Some(&attachment)).is_none());
    }
    let sent = sent.lock().unwrap();
    assert_eq!(sent[0], sent[1]);
    let frame: pale_ports::ResidentOperationFrameRequest =
        serde_json::from_slice(&sent[0]).unwrap();
    let wire: serde_json::Value = serde_json::from_str(frame.signed_common_request()).unwrap();
    let signature = signer.signature.unwrap();
    assert_eq!(wire["caller_signature"], base64url(&signature.to_bytes()));
    let key = signer.key.verifying_key();
    key.verify_strict(&signer.message, &signature).unwrap();
    let mut wrong_purpose = b"PALE_OPERATION_HANDLE_V1\0".to_vec();
    wrong_purpose.extend_from_slice(&signer.message[signer.message.len() - 32..]);
    assert!(key.verify_strict(&wrong_purpose, &signature).is_err());
    assert!(
        SigningKey::from_bytes(&[17; 32])
            .verifying_key()
            .verify_strict(&signer.message, &signature)
            .is_err(),
        "connection key cannot verify the request signature"
    );
}
