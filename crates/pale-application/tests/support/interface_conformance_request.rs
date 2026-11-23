//! Test-only variants of the admitted interface-conformance request.

use pale_core::records::{CommonRequestEnvelopeV1, RequestIdempotencyKeyV1};

pub(crate) fn conflicting_start_request(
    start: &CommonRequestEnvelopeV1,
) -> CommonRequestEnvelopeV1 {
    let mut value = start.to_preimage();
    value.idempotency_key =
        RequestIdempotencyKeyV1::parse("idempotency-interface-conformance-conflicting-start")
            .expect("conflicting Start idempotency identity");
    super::request::seal(&value)
}
