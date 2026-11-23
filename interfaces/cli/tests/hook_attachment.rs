//! Ordinary executable delivery over real sockets and public fixture credentials.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use ed25519_dalek::{Signer as _, SigningKey};
use pale_adapter_process::OsCryptographicRandom;
use pale_adapter_uds::{accept_authenticated_request, UnixSocketListener};
use pale_ports::{LocalConnectionSignerPort, LocalConnectionSigningError, LocalIpcFrameLimits};
use std::{
    fs,
    io::Write,
    num::NonZeroU32,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

struct Fixture {
    _temporary: tempfile::TempDir,
    configuration: PathBuf,
    endpoint: PathBuf,
    document: serde_json::Value,
}

fn public(seed: u8) -> [u8; 32] {
    SigningKey::from_bytes(&[seed; 32])
        .verifying_key()
        .to_bytes()
}

fn private_file(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().canonicalize().unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let endpoint = directory.join("resident.sock");
        let endpoint_configuration = directory.join("endpoint.json");
        let caller_credential = directory.join("caller.key");
        private_file(&caller_credential, &[17; 32]);
        private_file(
            &endpoint_configuration,
            &serde_json::to_vec(&serde_json::json!({"schema_version":1,"endpoint":endpoint}))
                .unwrap(),
        );
        let configuration = directory.join("hook.json");
        let document = serde_json::json!({
            "schema_version":1, "endpoint_configuration":endpoint_configuration,
            "caller_credential":caller_credential, "caller_public_key":public(17),
            "resident_public_key":public(29), "timeout_ms":2000
        });
        let fixture = Self {
            _temporary: temporary,
            configuration,
            endpoint,
            document,
        };
        fixture.save();
        fixture
    }

    fn save(&self) {
        private_file(
            &self.configuration,
            &serde_json::to_vec(&self.document).unwrap(),
        );
    }

    fn listener(&self) -> UnixSocketListener {
        UnixSocketListener::bind(
            &self.endpoint,
            LocalIpcFrameLimits::new(NonZeroU32::new(1 << 20).unwrap()),
        )
        .unwrap()
    }

    fn run(&self, payload: &[u8]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_pale"))
            .args([
                "hook",
                "session-start",
                "--workspace",
                "workspace-a",
                "--repository",
                "repository-a",
                "--provider-binding",
                "binding-a",
                "--provider-binding-identity",
                &"0".repeat(64),
                "--resident-hook-config",
            ])
            .arg(&self.configuration)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(payload).unwrap();
        child.wait_with_output().unwrap()
    }
}

struct ResidentSigner(SigningKey);
impl LocalConnectionSignerPort for ResidentSigner {
    fn public_key(&self) -> [u8; 32] {
        self.0.verifying_key().to_bytes()
    }
    fn sign(&mut self, bytes: &[u8], _: Instant) -> Result<[u8; 64], LocalConnectionSigningError> {
        Ok(self.0.sign(bytes).to_bytes())
    }
}

const NATIVE: &[u8] =
    b"{\n \"hook_event_name\":\"SessionStart\",\"session_id\":\"private-session\"\n}";

fn diagnostic(output: &Output) -> serde_json::Value {
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-session"));
    serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["diagnostic"].clone()
}

#[test]
fn executable_authenticates_and_preserves_the_resident_refusal() {
    let fixture = Fixture::new();
    let listener = fixture.listener();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut carrier = listener.accept(deadline).unwrap();
        let mut signer = ResidentSigner(SigningKey::from_bytes(&[29; 32]));
        let request = accept_authenticated_request(
            &mut carrier,
            &mut signer,
            &OsCryptographicRandom,
            deadline,
            |key| key == &public(17),
        )
        .unwrap();
        let frame: serde_json::Value = serde_json::from_slice(request.payload()).unwrap();
        request
            .respond(
                &mut carrier,
                &mut signer,
                br#"{"failure":"admission_denied","kind":"rejected"}"#,
            )
            .unwrap();
        frame
    });
    let output = fixture.run(NATIVE);
    assert_eq!(
        diagnostic(&output)["code"],
        "resident_hook_admission_denied"
    );
    let frame = server.join().unwrap();
    assert_eq!(frame["kind"], "hook");
    assert_eq!(frame["nativeEvent"].as_str().unwrap().as_bytes(), NATIVE);
    assert_eq!(frame["arguments"].as_array().unwrap().len(), 10);
}

#[test]
fn wrong_resident_sends_no_event_and_a_lost_reply_stays_uncertain() {
    for wrong_peer in [true, false] {
        let mut fixture = Fixture::new();
        if wrong_peer {
            fixture.document["resident_public_key"] = serde_json::json!(public(31));
            fixture.save();
        }
        let listener = fixture.listener();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut carrier = listener.accept(deadline).unwrap();
            let mut signer = ResidentSigner(SigningKey::from_bytes(&[29; 32]));
            let request = accept_authenticated_request(
                &mut carrier,
                &mut signer,
                &OsCryptographicRandom,
                deadline,
                |key| key == &public(17),
            );
            assert_eq!(request.is_err(), wrong_peer);
            // Drop a received request without returning a reply. No retry follows.
            assert!(listener.try_accept().unwrap().is_none());
        });
        let output = fixture.run(NATIVE);
        let value = diagnostic(&output);
        assert_eq!(
            value["code"],
            if wrong_peer {
                "resident_hook_not_sent"
            } else {
                "resident_hook_outcome_unknown"
            }
        );
        server.join().unwrap();
    }
}

#[test]
fn invalid_configuration_and_input_never_connect() {
    for case in 0..7 {
        let mut fixture = Fixture::new();
        let listener = fixture.listener();
        match case {
            0 => fixture.document["timeout_ms"] = serde_json::json!(0),
            1 => fixture.document["extra"] = serde_json::json!(true),
            2 => fixture.document["caller_public_key"] = serde_json::json!(public(31)),
            3 => fixture.document["caller_credential"] = serde_json::json!("relative.key"),
            _ => (),
        }
        fixture.save();
        if case == 4 {
            fs::set_permissions(&fixture.configuration, fs::Permissions::from_mode(0o644)).unwrap();
        }
        if case == 5 {
            fs::remove_file(&fixture.configuration).unwrap();
        }
        let output = fixture.run(if case == 6 { b"{" } else { NATIVE });
        diagnostic(&output);
        assert!(listener.try_accept().unwrap().is_none());
    }
}

#[test]
fn executable_reaches_the_actual_resident_application() {
    use pale_adapter_uds::UnixSocketCarrier;
    use pale_application::resident::{
        with_in_memory_invoke_conformance_frontier_for_peer, ResidentWorkTurnOutcome,
    };
    use pale_ports::{
        LocalIpcFramePort, ResidentOperationFrameLocator, ResidentOperationFrameResponse,
    };
    use pale_resident::{
        ConnectionDeadlines, ResidentConnectionAuthentication, ResidentHostTurn, ResidentLocalHost,
    };
    use pale_sdk::AuthenticatedResidentCarrierSource as _;
    let later = || Instant::now() + Duration::from_secs(10);
    let limits = || LocalIpcFrameLimits::new(NonZeroU32::new(1 << 20).unwrap());
    let peer_fixture = Fixture::new();
    let listener = peer_fixture.listener();
    let _peer = UnixSocketCarrier::connect(&peer_fixture.endpoint, limits(), later()).unwrap();
    let peer = listener
        .accept(later())
        .unwrap()
        .peer_credentials()
        .unwrap();
    with_in_memory_invoke_conformance_frontier_for_peer(
        peer,
        |service, requests| {
            let fixture = Fixture::new();
            let mut signer = ResidentSigner(SigningKey::from_bytes(&[29; 32]));
            let allowed = |key: &[u8; 32]| key == &public(17);
            let mut host = ResidentLocalHost::bind(
                service,
                &fixture.endpoint,
                limits(),
                std::num::NonZeroUsize::new(2).unwrap(),
                ResidentConnectionAuthentication::new(
                    &mut signer,
                    &OsCryptographicRandom,
                    &allowed,
                ),
            )
            .unwrap();
            let request = requests.start_signed_common_request().to_owned();
            let locator = std::thread::scope(|scope| {
                let client = scope.spawn(|| {
                    let mut source = pale_sdk::LocalAgentCarrierSource::load(
                        fixture.document["endpoint_configuration"].as_str().unwrap(),
                        fixture.document["caller_credential"].as_str().unwrap(),
                        public(17),
                        public(29),
                    )
                    .unwrap();
                    let mut carrier = source.open(later()).unwrap();
                    let bytes = serde_json::to_vec(
                        &serde_json::json!({"kind":"start_invoke","signedCommonRequest":request}),
                    )
                    .unwrap();
                    let response = carrier.exchange(&bytes, later()).unwrap();
                    let ResidentOperationFrameResponse::Started { records } =
                        serde_json::from_slice(&response).unwrap()
                    else {
                        panic!("fixture operation must start")
                    };
                    let handle: serde_json::Value =
                        serde_json::from_str(records.operation_handle_canonical()).unwrap();
                    let locator = ResidentOperationFrameLocator::new(
                        records.operation_handle_record_ref().to_owned(),
                        handle["canonical_operation_handle_sha256"]
                            .as_str()
                            .unwrap()
                            .to_owned(),
                    );
                    let output = fixture.run(NATIVE);
                    assert_eq!(
                        diagnostic(&output)["code"],
                        "resident_hook_admission_denied"
                    );
                    locator
                });
                let deadline = later();
                let mut frames = 0;
                while frames < 2 {
                    assert!(Instant::now() < deadline);
                    if matches!(
                        host.run_one(Some(ConnectionDeadlines::new(deadline, deadline)), true)
                            .unwrap(),
                        ResidentHostTurn::Work(ResidentWorkTurnOutcome::Frame(_))
                    ) {
                        frames += 1;
                    }
                    std::thread::yield_now();
                }
                client.join().unwrap()
            });
            (locator, ())
        },
        |_, _, _, ()| {},
    )
    .unwrap();
}
