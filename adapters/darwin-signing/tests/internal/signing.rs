//! Policy refusals happen before native lookup; cryptographic evidence stays typed.
use super::*;
use ed25519_dalek::{Signature, Verifier};
use serde_json::json;

fn request() -> Request {
    let public = SigningKey::from_bytes(&[7; 32]).verifying_key();
    serde_json::from_value(json!({"schema_version":1,"kind":"operation_handle",
        "digest":"a".repeat(64),"service":"fixture-service","account":"fixture-account",
        "public_key":URL_SAFE_NO_PAD.encode(public.as_bytes()),"budget_ms":1000}))
    .unwrap()
}

#[test]
fn signature_authenticates_exact_core_domain_and_digest() {
    let request = request();
    let signature = sign_with(
        &request,
        &Mutex::new(()),
        Instant::now() + Duration::from_secs(1),
        |_| Ok(Zeroizing::new(vec![7; 32])),
    )
    .unwrap_or_else(|_| panic!("valid synthetic key"));
    let key = SigningKey::from_bytes(&[7; 32]).verifying_key();
    let mut message = OPERATION_HANDLE_V1_SIGNATURE_DOMAIN.to_vec();
    message.extend_from_slice(&[0xaa; 32]);
    assert!(key
        .verify(&message, &Signature::from_bytes(&signature))
        .is_ok());
    message[0] ^= 1;
    assert!(key
        .verify(&message, &Signature::from_bytes(&signature))
        .is_err());
    message[0] ^= 1;
    *message.last_mut().unwrap() ^= 1;
    assert!(key
        .verify(&message, &Signature::from_bytes(&signature))
        .is_err());
}

#[test]
fn invalid_expired_and_busy_requests_never_read_a_key() {
    let gate = Mutex::new(());
    let mut request = request();
    let never = |_: &Request| -> Result<Zeroizing<Vec<u8>>, Refusal> {
        panic!("native lookup must not run")
    };
    request.digest = "A".repeat(64);
    assert!(matches!(
        sign_with(
            &request,
            &gate,
            Instant::now() + Duration::from_secs(1),
            never
        ),
        Err(Refusal::InvalidRequest)
    ));
    request.digest = "a".repeat(64);
    assert!(matches!(
        sign_with(&request, &gate, Instant::now(), never),
        Err(Refusal::Deadline)
    ));
    let _held = gate.lock().unwrap();
    assert!(matches!(
        sign_with(
            &request,
            &gate,
            Instant::now() + Duration::from_secs(1),
            never
        ),
        Err(Refusal::Busy)
    ));
}

#[test]
fn wrong_key_short_seed_and_late_lookup_are_refused() {
    let request = request();
    let gate = Mutex::new(());
    assert!(matches!(
        sign_with(
            &request,
            &gate,
            Instant::now() + Duration::from_secs(1),
            |_| Ok(Zeroizing::new(vec![8; 32]))
        ),
        Err(Refusal::WrongKey)
    ));
    assert!(matches!(
        sign_with(
            &request,
            &gate,
            Instant::now() + Duration::from_secs(1),
            |_| Ok(Zeroizing::new(vec![7; 31]))
        ),
        Err(Refusal::KeyUnavailable)
    ));
    let deadline = Instant::now() + Duration::from_millis(5);
    assert!(matches!(
        sign_with(&request, &gate, deadline, |_| {
            std::thread::sleep(Duration::from_millis(10));
            Ok(Zeroizing::new(vec![7; 32]))
        }),
        Err(Refusal::Deadline)
    ));
}

#[test]
fn native_boundary_refuses_oversize_unknown_fields_and_unlisted_kinds() {
    let input = SigningBridgeRequest {
        length: 4097,
        bytes: [0; 4096],
    };
    assert_eq!(pale_darwin_signing_execute_v1(input).status, 1);
    let complete = json!({"schema_version":1,"kind":"operation_handle",
        "digest":"a".repeat(64),"service":"fixture-service","account":"fixture-account",
        "public_key":URL_SAFE_NO_PAD.encode(SigningKey::from_bytes(&[7;32]).verifying_key().as_bytes()),
        "budget_ms":1000});
    assert!(serde_json::from_value::<Request>(complete.clone()).is_ok());
    for (field, value) in [
        ("kind", json!("arbitrary_domain")),
        ("seed", json!("forbidden")),
    ] {
        let mut invalid = complete.clone();
        invalid[field] = value;
        // The exact boundary parser is tested without invoking native lookup,
        // even if a future regression were to accept the malformed request.
        assert!(
            serde_json::from_value::<Request>(invalid).is_err(),
            "{field}"
        );
    }
}
