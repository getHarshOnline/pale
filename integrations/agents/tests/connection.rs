//! Actual SDK and resident listener connections, with explicit fixture authority.
#![cfg(any(target_os = "linux", target_os = "macos"))]

mod support;
use pale_adapter_uds::{accept_authenticated_request, UnixSocketCarrier, UnixSocketListener};
use pale_application::resident::{
    with_in_memory_invoke_conformance_frontier_for_peer, ResidentWorkTurnOutcome,
};
use pale_ports::{
    LocalConnectionSignerPort, LocalIpcFramePort, ResidentOperationFrameLocator,
    ResidentOperationFrameResponse,
};
use pale_resident::{
    ConnectionDeadlines, ResidentConnectionAuthentication, ResidentHostTurn, ResidentLocalHost,
};
use pale_sdk::{
    AuthenticatedResidentCarrierFailure, AuthenticatedResidentCarrierSource, DiagnosticDisposition,
    DiagnosticEffectCertainty, PaleApi, ResidentAttachment, ResidentHookRequest,
};
use std::{
    fs,
    num::NonZeroUsize,
    time::{Duration, Instant},
};
use support::*;

#[test]
fn sdk_hook_reaches_the_real_resident_listener_and_preserves_its_denial() {
    let peer_fixture = Fixture::new();
    let listener = UnixSocketListener::bind(&peer_fixture.endpoint, limits()).unwrap();
    let _client = UnixSocketCarrier::connect(&peer_fixture.endpoint, limits(), later()).unwrap();
    let peer = listener
        .accept(later())
        .unwrap()
        .peer_credentials()
        .unwrap();
    with_in_memory_invoke_conformance_frontier_for_peer(peer, |service, requests| {
        let fixture = Fixture::new();
        let mut signer = FixtureSigner::resident();
        let resident_key = signer.public_key();
        let caller_key = FixtureSigner::caller().public_key();
        let selected = |key: &[u8; 32]| key == &caller_key;
        let mut host = ResidentLocalHost::bind(service, &fixture.endpoint, limits(), NonZeroUsize::new(2).unwrap(), ResidentConnectionAuthentication::new(&mut signer, &OsEntropy, &selected)).unwrap();
        let signed_start = requests.start_signed_common_request().to_owned();
        let mut start_source = fixture.source(resident_key);
        let hook_source = fixture.source(resident_key);
        let locator = std::thread::scope(|scope| {
            let client = scope.spawn(move || {
                let mut carrier = start_source.open(later()).unwrap();
                let bytes = serde_json::to_vec(&serde_json::json!({"kind":"start_invoke", "signedCommonRequest":signed_start})).unwrap();
                let response = carrier.exchange(&bytes, later()).unwrap();
                let ResidentOperationFrameResponse::Started { records } = serde_json::from_slice(&response).unwrap() else { panic!("actual operation must start") };
                let handle: serde_json::Value = serde_json::from_str(records.operation_handle_canonical()).unwrap();
                let locator = ResidentOperationFrameLocator::new(records.operation_handle_record_ref().to_owned(), handle["canonical_operation_handle_sha256"].as_str().unwrap().to_owned());
                let attachment = ResidentAttachment::provision_hooks(Box::new(hook_source), Duration::from_secs(5)).unwrap();
                let api = PaleApi::new().unwrap();
                let event = br#"{"hook_event_name":"SessionStart","session_id":"native-session"}"#.to_vec();
                let request = ResidentHookRequest::new("session-start".into(), "workspace-a".into(), "repository-a".into(), "binding-a".into(), "0".repeat(64), event);
                let error = api.deliver_resident_hook(Some(&attachment), api.admit_resident_hook(request).unwrap()).unwrap_err();
                assert_eq!(error.diagnostic().code(), "resident_hook_admission_denied");
                assert_eq!(error.diagnostic().disposition(), DiagnosticDisposition::Blocked);
                assert_eq!(error.diagnostic().effect(), DiagnosticEffectCertainty::NotStarted);
                locator
            });
            let until = later();
            let mut frames = 0;
            while frames < 2 {
                assert!(Instant::now() < until);
                if matches!(host.run_one(Some(ConnectionDeadlines::new(until, until)), true).unwrap(), ResidentHostTurn::Work(ResidentWorkTurnOutcome::Frame(_))) { frames += 1; }
                std::thread::yield_now();
            }
            client.join().unwrap()
        });
        (locator, ())
    }, |_, _, _, ()| {}).unwrap();
}

#[test]
fn separate_requests_reauthenticate_and_a_replaced_endpoint_parent_is_refused() {
    let fixture = Fixture::new();
    let listener = UnixSocketListener::bind(&fixture.endpoint, limits()).unwrap();
    let mut source = fixture.source(FixtureSigner::resident().public_key());
    let server = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for _ in 0..2 {
            let mut carrier = listener.accept(later()).unwrap();
            let mut signer = FixtureSigner::resident();
            let request = accept_authenticated_request(
                &mut carrier,
                &mut signer,
                &OsEntropy,
                later(),
                |key| key == &FixtureSigner::caller().public_key(),
            )
            .unwrap();
            seen.push(request.payload().to_vec());
            request
                .respond(&mut carrier, &mut signer, b"response")
                .unwrap();
        }
        seen
    });
    let mut carrier = source.open(later()).unwrap();
    for request in [b"first".as_slice(), b"second".as_slice()] {
        assert_eq!(&*carrier.exchange(request, later()).unwrap(), b"response");
    }
    assert_eq!(
        server.join().unwrap(),
        vec![b"first".to_vec(), b"second".to_vec()]
    );
    fs::rename(
        &fixture.directory,
        fixture.directory.with_extension("retired"),
    )
    .unwrap();
    fs::create_dir(&fixture.directory).unwrap();
    assert_eq!(
        carrier.exchange(b"third", later()).unwrap_err(),
        AuthenticatedResidentCarrierFailure::ConnectionUnavailable
    );
}

#[test]
fn wrong_peer_releases_no_payload_and_lost_response_stays_uncertain() {
    for wrong_peer in [true, false] {
        let fixture = Fixture::new();
        let listener = UnixSocketListener::bind(&fixture.endpoint, limits()).unwrap();
        let pinned = if wrong_peer {
            FixtureSigner::caller().public_key()
        } else {
            FixtureSigner::resident().public_key()
        };
        let mut source = fixture.source(pinned);
        let server = std::thread::spawn(move || {
            let mut carrier = listener.accept(later()).unwrap();
            accept_authenticated_request(
                &mut carrier,
                &mut FixtureSigner::resident(),
                &OsEntropy,
                later(),
                |_| true,
            )
            .map(|request| request.payload().to_vec())
        });
        if wrong_peer {
            assert!(source.open(later()).is_err());
        } else {
            let mut carrier = source.open(later()).unwrap();
            assert_eq!(
                carrier.exchange(b"once", later()).unwrap_err(),
                AuthenticatedResidentCarrierFailure::OutcomeUnknown
            );
        }
        let observed = server.join().unwrap();
        if wrong_peer {
            assert!(observed.is_err());
        } else {
            assert_eq!(observed.unwrap(), b"once");
        }
    }
}
