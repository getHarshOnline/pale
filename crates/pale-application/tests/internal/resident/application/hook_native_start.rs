//! Native starts retain the original caller while durable issuance completes.

use super::*;
use crate::resident::process_host::native_hook_start::{NativeHookStartContext, NativeHookStarts};
use crate::resident::ResidentNativeToolStart;
use pale_ports::ResidentOperationFrameFailure as Failure;

#[test]
fn native_start_retains_consumed_operation_across_interrupted_hook_turns() -> TestResult {
    with_tool_mapping_and_requests(
        NativeMapping::Read,
        &["call-1"],
        |app, history, _, current, _, admitted, operations, route, requests| {
            let context = || NativeHookStartContext {
                application: app,
                session: admitted.session(),
                historical: history,
                current,
            };
            let mut owner = NativeHookStarts::new(NonZeroUsize::new(1).unwrap());
            owner
                .reserve(
                    &requests[0],
                    ResidentNativeToolStart::admit(requests[0].body())?,
                    HOOK_CALLER_KEY,
                    context(),
                    &mut ProvingAuthority,
                )
                .unwrap();
            owner
                .issued(
                    &requests[0],
                    &operations[0],
                    context(),
                    &mut ProvingAuthority,
                )
                .unwrap();
            let bytes = requests[0].body().payload["native_event"].as_str().unwrap();
            let original = captured_hook(route, bytes, HOOK_CALLER_KEY);
            let first = owner
                .continue_hook(&original, context(), &mut ProvingAuthority)
                .unwrap();
            assert_eq!(first.operation().handle(), operations[0].handle());
            first.admit_native_tool(app)?;
            drop(first); // The next owner can fail without stranding consumption.
            let changed = captured_hook(route, &event("call-1", "changed"), HOOK_CALLER_KEY);
            assert_eq!(
                owner.original_request(&changed).unwrap_err(),
                Failure::AdmissionDenied
            );
            let foreign = captured_hook(route, bytes, [99; 32]);
            assert_eq!(
                owner.original_request(&foreign).unwrap_err(),
                Failure::AdmissionDenied
            );
            owner
                .issued(
                    &requests[0],
                    &operations[0],
                    context(),
                    &mut ProvingAuthority,
                )
                .unwrap();
            let retried = owner
                .continue_hook(&original, context(), &mut ProvingAuthority)
                .unwrap();
            assert_eq!(retried.operation().handle(), operations[0].handle());
            let expired = current.at_time(operations[0].historical_operation_lease().expires_at());
            assert!(owner
                .continue_hook(
                    &original,
                    NativeHookStartContext {
                        current: &expired,
                        ..context()
                    },
                    &mut ProvingAuthority
                )
                .is_err());
            Ok(())
        },
    )
}

#[test]
fn native_start_reservation_survives_replay_without_rebinding_or_reissuing() -> TestResult {
    with_tool_mapping_and_requests(
        NativeMapping::Read,
        &["call-1", "call-2"],
        |app, history, _, current, _, admitted, operations, _, requests| {
            let context = || NativeHookStartContext {
                application: app,
                session: admitted.session(),
                historical: history,
                current,
            };
            let mut owner = NativeHookStarts::new(NonZeroUsize::new(1).unwrap());
            let reserve = |owner: &mut NativeHookStarts, request: &CommonRequestEnvelopeV1, key| {
                owner.reserve(
                    request,
                    ResidentNativeToolStart::admit(request.body()).unwrap(),
                    key,
                    context(),
                    &mut ProvingAuthority,
                )
            };
            reserve(&mut owner, &requests[0], HOOK_CALLER_KEY).unwrap();
            reserve(&mut owner, &requests[0], HOOK_CALLER_KEY).unwrap();
            assert_eq!(
                reserve(&mut owner, &requests[0], [99; 32]),
                Err(Failure::ReplayConflict)
            );
            assert_eq!(
                reserve(&mut owner, &requests[1], HOOK_CALLER_KEY),
                Err(Failure::ResidentUnavailable)
            );
            assert_eq!(
                owner.issued(
                    &requests[0],
                    &operations[1],
                    context(),
                    &mut ProvingAuthority
                ),
                Err(Failure::ReplayConflict)
            );
            owner
                .issued(
                    &requests[0],
                    &operations[0],
                    context(),
                    &mut ProvingAuthority,
                )
                .unwrap();
            // A second response for the same start must preserve the registry's
            // existing entry rather than attempt duplicate registration.
            owner
                .issued(
                    &requests[0],
                    &operations[0],
                    context(),
                    &mut ProvingAuthority,
                )
                .unwrap();
            assert_eq!(
                reserve(&mut owner, &requests[0], [99; 32]),
                Err(Failure::ReplayConflict)
            );
            Ok(())
        },
    )
}

#[test]
fn native_start_requires_the_signed_body_to_match_its_session_route() -> TestResult {
    with_tool_mapping_and_requests(
        NativeMapping::Read,
        &["call-1"],
        |app, history, _, current, _, admitted, _, _, requests| {
            let mut preimage = requests[0].to_preimage();
            preimage.body.payload["repository_id"] = "foreign-repository".into();
            preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
                pale_core::canon::to_canonical_string(&preimage.body.payload)?.as_bytes(),
            );
            let request = crate::effect_case_lineage_fixture::request(&preimage);
            let mut owner = NativeHookStarts::new(NonZeroUsize::new(1).unwrap());
            let result = owner.reserve(
                &request,
                ResidentNativeToolStart::admit(request.body())?,
                HOOK_CALLER_KEY,
                NativeHookStartContext {
                    application: app,
                    session: admitted.session(),
                    historical: history,
                    current,
                },
                &mut ProvingAuthority,
            );
            assert_eq!(result, Err(Failure::AdmissionDenied));
            Ok(())
        },
    )
}
