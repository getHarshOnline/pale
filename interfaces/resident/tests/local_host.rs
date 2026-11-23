//! Native sockets around the application-owned in-memory conformance service.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::{
    io::Write,
    num::{NonZeroU32, NonZeroUsize},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::Path,
    time::{Duration, Instant},
};

use pale_adapter_uds::{LocalEndpointConfiguration, UnixSocketCarrier, UnixSocketListener};
use pale_application::resident::{
    with_in_memory_invoke_conformance_frontier_for_peer, ResidentWorkTurnOutcome,
};
use pale_ports::{
    LocalIpcFrameLimits, LocalIpcFramePort, LocalIpcPeerCredentials, ResidentHookFrameRequest,
    ResidentHookFrameResponse, ResidentObserveFrameRequest, ResidentObserveFrameResponse,
    ResidentOperationFrameFailure, ResidentOperationFrameLocator, ResidentOperationFrameRequest,
    ResidentOperationFrameResponse,
};
use pale_resident::{
    ConnectionDeadlines, ResidentConnectionAuthentication, ResidentHostError, ResidentHostTurn,
    ResidentLocalHost,
};

#[path = "local_host/authentication.rs"]
mod authentication;
use authentication::{selected_caller, FixtureSigner, OsEntropy};

fn limits() -> LocalIpcFrameLimits {
    LocalIpcFrameLimits::new(NonZeroU32::new(1 << 20).unwrap())
}

fn later() -> Instant {
    Instant::now() + Duration::from_secs(3)
}

fn deadlines() -> ConnectionDeadlines {
    ConnectionDeadlines::new(later(), later())
}

fn observed_peer() -> LocalIpcPeerCredentials {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("peer.sock");
    let listener = UnixSocketListener::bind(&path, limits()).unwrap();
    let _client = UnixSocketCarrier::connect(&path, limits(), later()).unwrap();
    listener
        .accept(later())
        .unwrap()
        .peer_credentials()
        .unwrap()
}

fn exchange(host: &mut ResidentLocalHost<'_, '_>, path: &Path, bytes: &[u8]) -> Vec<u8> {
    std::thread::scope(|scope| {
        let client = scope.spawn(|| {
            let mut carrier = UnixSocketCarrier::connect(path, limits(), later()).unwrap();
            pale_adapter_uds::exchange_authenticated(
                &mut carrier,
                &mut FixtureSigner::caller(),
                FixtureSigner::resident_key(),
                &OsEntropy,
                bytes,
                later(),
            )
            .unwrap()
            .into_vec()
        });
        let until = later();
        let mut completed = false;
        while Instant::now() < until && !completed {
            let outcome = host.run_one(Some(deadlines()), true).unwrap();
            completed = matches!(
                outcome,
                ResidentHostTurn::Work(ResidentWorkTurnOutcome::Frame(_))
            );
            std::thread::yield_now();
        }
        assert!(completed, "ready request must not starve behind recovery");
        client.join().unwrap()
    })
}

#[test]
fn real_socket_start_and_cold_observe_preserve_the_application_handle() {
    let peer = observed_peer();
    assert_eq!(peer.process_id(), Some(std::process::id()));
    with_in_memory_invoke_conformance_frontier_for_peer(
        peer,
        |service, requests| {
            let mut signer = FixtureSigner::resident();
            let temp = tempfile::tempdir().unwrap();
            let occupied = temp.path().join("occupied");
            std::fs::write(&occupied, b"existing owner").unwrap();
            assert!(matches!(
                ResidentLocalHost::bind(
                    service,
                    &occupied,
                    limits(),
                    NonZeroUsize::new(2).unwrap(),
                    ResidentConnectionAuthentication::new(
                        &mut signer,
                        &OsEntropy,
                        &selected_caller
                    ),
                ),
                Err(ResidentHostError::Carrier(_))
            ));
            assert_eq!(std::fs::read(&occupied).unwrap(), b"existing owner");

            let private = temp.path().canonicalize().unwrap();
            std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
            let path = private.join("resident.sock");
            let config = private.join("machine.json");
            std::fs::write(
                &config,
                serde_json::to_vec(&serde_json::json!({"schema_version":1,"endpoint":path}))
                    .unwrap(),
            )
            .unwrap();
            std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600)).unwrap();
            let configuration = LocalEndpointConfiguration::load(&config).unwrap();
            let mut host = ResidentLocalHost::bind_configured(
                service,
                &configuration,
                limits(),
                NonZeroUsize::new(2).unwrap(),
                ResidentConnectionAuthentication::new(&mut signer, &OsEntropy, &selected_caller),
            )
            .unwrap();
            assert_eq!(host.run_one(None, false).unwrap(), ResidentHostTurn::Idle);
            assert!(matches!(
                host.run_one(
                    Some(ConnectionDeadlines::new(Instant::now(), later())),
                    false
                ),
                Err(ResidentHostError::ExpiredOffer)
            ));

            let invalid = ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: "{}".into(),
            };
            let response = exchange(&mut host, &path, &serde_json::to_vec(&invalid).unwrap());
            assert!(matches!(
                serde_json::from_slice::<ResidentOperationFrameResponse>(&response).unwrap(),
                ResidentOperationFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::InvalidRequest
                }
            ));

            let hook = ResidentHookFrameRequest::new(
                [
                    "hook".into(), "pre-tool-use".into(),
                    "--workspace".into(), "ws.main".into(),
                    "--repository".into(), "repo.main".into(),
                    "--provider-binding".into(), "terminal-provider".into(),
                    "--provider-binding-identity".into(), "a".repeat(64),
                ],
                r#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_name":"Read"}"#.into(),
            );
            let hook = serde_json::to_value(hook).unwrap();
            let response = exchange(&mut host, &path, &serde_json::to_vec(&hook).unwrap());
            assert_eq!(
                serde_json::from_slice::<ResidentHookFrameResponse>(&response).unwrap(),
                ResidentHookFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::AdmissionDenied,
                },
                "authenticated hook input cannot create operation authority"
            );

            let quiet = UnixStream::connect(&path).unwrap();
            assert_eq!(
                host.run_one(Some(deadlines()), false).unwrap(),
                ResidentHostTurn::AwaitingFrame
            );
            let request = ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: requests.start_signed_common_request().to_owned(),
            };
            let response = exchange(&mut host, &path, &serde_json::to_vec(&request).unwrap());
            let ResidentOperationFrameResponse::Started { records } =
                serde_json::from_slice(&response).unwrap()
            else {
                panic!("application must establish the durable start")
            };
            let handle = records.operation_handle_canonical().as_bytes().to_vec();
            let value: serde_json::Value = serde_json::from_slice(&handle).unwrap();
            let locator = ResidentOperationFrameLocator::new(
                records.operation_handle_record_ref().to_owned(),
                value["canonical_operation_handle_sha256"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
            drop(quiet);
            drop(host);
            assert!(
                path.exists(),
                "transport drop must not unlink an endpoint by name"
            );
            (locator, handle)
        },
        |service, requests, locator, handle| {
            let mut signer = FixtureSigner::resident();
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("observe.sock");
            let mut host = ResidentLocalHost::bind(
                service,
                &path,
                limits(),
                NonZeroUsize::new(2).unwrap(),
                ResidentConnectionAuthentication::new(&mut signer, &OsEntropy, &selected_caller),
            )
            .unwrap();
            let request = ResidentObserveFrameRequest::new(
                locator.operation_handle_record_ref().to_owned(),
                locator.operation_handle_sha256().to_owned(),
                requests
                    .observe_signed_common_request(locator, &handle)
                    .unwrap(),
            );
            let response = exchange(&mut host, &path, &serde_json::to_vec(&request).unwrap());
            let ResidentObserveFrameResponse::Observed {
                operation_handle_record_ref,
                operation_handle_canonical,
                ..
            } = serde_json::from_slice(&response).unwrap()
            else {
                panic!("cold-recovered application must observe the exact operation")
            };
            assert_eq!(
                operation_handle_record_ref,
                locator.operation_handle_record_ref()
            );
            assert_eq!(operation_handle_canonical.as_bytes(), &handle);
        },
    )
    .unwrap();
}

#[test]
fn pending_deadlines_do_not_renew_and_partial_frame_failure_allows_a_new_connection() {
    with_in_memory_invoke_conformance_frontier_for_peer(
        observed_peer(),
        |service, requests| {
            let mut signer = FixtureSigner::resident();
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("resident.sock");
            let mut host = ResidentLocalHost::bind(
                service,
                &path,
                limits(),
                NonZeroUsize::new(2).unwrap(),
                ResidentConnectionAuthentication::new(&mut signer, &OsEntropy, &selected_caller),
            )
            .unwrap();
            let client = UnixStream::connect(&path).unwrap();
            let original = Instant::now() + Duration::from_millis(60);
            assert_eq!(
                host.run_one(Some(ConnectionDeadlines::new(original, original)), false)
                    .unwrap(),
                ResidentHostTurn::AwaitingFrame
            );
            assert!(matches!(
                host.run_one(Some(deadlines()), true).unwrap(),
                ResidentHostTurn::Work(
                    ResidentWorkTurnOutcome::RecoveryAdvanced
                        | ResidentWorkTurnOutcome::RecoveryDeferred
                )
            ));
            std::thread::sleep(Duration::from_millis(90));
            assert_eq!(
                host.run_one(Some(deadlines()), false).unwrap(),
                ResidentHostTurn::ConnectionExpired
            );
            drop(client);

            let mut partial = UnixStream::connect(&path).unwrap();
            partial.write_all(&[0, 0]).unwrap();
            let frame = Instant::now() + Duration::from_millis(30);
            let mut failed = false;
            for _ in 0..4 {
                match host.run_one(Some(ConnectionDeadlines::new(frame, later())), false) {
                    Err(ResidentHostError::Authentication(_)) => {
                        failed = true;
                        break;
                    }
                    Ok(_) => {}
                    other => panic!("unexpected partial-frame outcome: {other:?}"),
                }
            }
            assert!(failed, "partial frame must end at its original bound");
            drop(partial);

            let request = ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: requests.start_signed_common_request().to_owned(),
            };
            let response = exchange(&mut host, &path, &serde_json::to_vec(&request).unwrap());
            let ResidentOperationFrameResponse::Started { records } =
                serde_json::from_slice(&response).unwrap()
            else {
                panic!("fresh connection must still work")
            };
            let value: serde_json::Value =
                serde_json::from_str(records.operation_handle_canonical()).unwrap();
            (
                ResidentOperationFrameLocator::new(
                    records.operation_handle_record_ref().to_owned(),
                    value["canonical_operation_handle_sha256"]
                        .as_str()
                        .unwrap()
                        .into(),
                ),
                (),
            )
        },
        |_, _, _, ()| {},
    )
    .unwrap();
}
