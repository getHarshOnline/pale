//! The core signing boundary uses modeled signatures; native crypto is tested by its owner.

use super::*;

#[test]
fn common_request_candidate_signature_does_not_admit_a_substituted_caller() {
    let fixture = action_request_fixture(&action());
    let mut candidate = fixture.preimage.clone();
    candidate.caller.identity = RequestCallerIdV1::parse("another-caller").unwrap();
    let signed = CommonRequestEnvelopeWireV1::sign_candidate(candidate, |purpose, _, _| {
        assert_eq!(purpose, COMMON_REQUEST_ENVELOPE_V1_SIGNATURE_DOMAIN);
        Ok(signature())
    })
    .unwrap();
    let refused = CommonRequestEnvelopeV1::admit_contextual_wire(
        signed,
        &request_context(
            &fixture.preimage,
            Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
        ),
        |_, _, _| panic!("the caller must be admitted before signature verification"),
    );
    assert!(refused.is_err());
}

#[test]
fn common_request_candidate_rejects_a_changed_payload_before_signing() {
    let fixture = action_request_fixture(&action());
    let mut candidate = fixture.preimage;
    candidate.body.payload = serde_json::json!({"substituted": true});
    assert!(
        CommonRequestEnvelopeWireV1::sign_candidate(candidate, |_, _, _| {
            panic!("malformed payload must not reach the signer")
        })
        .is_err()
    );
}

#[test]
fn common_request_issuance_passes_the_exact_purpose_caller_and_digest_once() {
    let fixture = action_request_fixture(&action());
    let expected = &fixture.preimage;
    let context = request_context(
        expected,
        Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
    );
    let mut calls = 0;
    let issued = CommonRequestEnvelopeV1::issue_contextual(
        expected.clone(),
        &context,
        |purpose, preimage, digest| {
            calls += 1;
            assert_eq!(purpose, b"PALE_COMMON_REQUEST_ENVELOPE_V1\0");
            assert_ne!(purpose, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
            assert_eq!(preimage, expected);
            assert_eq!(
                digest,
                &CommonRequestEnvelopeDigest::hash_canonical(expected)?
            );
            Ok(signature())
        },
    )
    .unwrap();
    assert_eq!(calls, 1);
    CommonRequestEnvelopeV1::admit_contextual_wire(
        common_request_wire(&issued),
        &context,
        |preimage, digest, signed| {
            assert_eq!(preimage, expected);
            assert_eq!(digest, issued.canonical_request_sha256());
            assert_eq!(signed, &signature());
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn common_request_issuance_preserves_a_signers_purpose_refusal() {
    let fixture = action_request_fixture(&action());
    let refused = InvalidResidentRecordV1::ContextualAdmission {
        record: "request signing",
        invariant: "selected key is not authorized for this purpose",
    };
    let result = CommonRequestEnvelopeV1::issue_contextual(
        fixture.preimage.clone(),
        &request_context(
            &fixture.preimage,
            Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
        ),
        |_, _, _| Err(refused),
    );
    assert!(matches!(
        result,
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "request signing",
            invariant: "selected key is not authorized for this purpose"
        })
    ));
}
