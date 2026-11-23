//! Fixture credentials never enter native deployment configuration.

use std::time::Instant;

use ed25519_dalek::{Signer as _, SigningKey};
use pale_ports::{
    CryptographicRandomSourceError, CryptographicRandomSourcePort, LocalConnectionSignerPort,
    LocalConnectionSigningError,
};

pub(super) struct FixtureSigner(SigningKey);

impl FixtureSigner {
    pub(super) fn resident() -> Self {
        Self(SigningKey::from_bytes(&[29; 32]))
    }
    pub(super) fn caller() -> Self {
        Self(SigningKey::from_bytes(&[17; 32]))
    }
    pub(super) fn resident_key() -> [u8; 32] {
        Self::resident().public_key()
    }
}

pub(super) fn selected_caller(key: &[u8; 32]) -> bool {
    *key == FixtureSigner::caller().public_key()
}

impl LocalConnectionSignerPort for FixtureSigner {
    fn public_key(&self) -> [u8; 32] {
        self.0.verifying_key().to_bytes()
    }
    fn sign(
        &mut self,
        message: &[u8],
        deadline: Instant,
    ) -> Result<[u8; 64], LocalConnectionSigningError> {
        if Instant::now() >= deadline {
            return Err(LocalConnectionSigningError);
        }
        Ok(self.0.sign(message).to_bytes())
    }
}

pub(super) struct OsEntropy;

impl CryptographicRandomSourcePort for OsEntropy {
    fn fill_bytes(&self, bytes: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        getrandom::fill(bytes).map_err(|_| CryptographicRandomSourceError::unavailable())
    }
}

use super::{deadlines, exchange, later, limits, observed_peer};
use pale_adapter_uds::{
    exchange_authenticated, LocalAuthenticationError, LocalRequestDelivery, UnixSocketCarrier,
};
use pale_application::resident::{
    with_in_memory_invoke_conformance_frontier_for_peer, ResidentWorkTurnOutcome,
};
use pale_ports::{
    LocalIpcCarrierError, LocalIpcFramePort, LocalIpcPeerCredentials,
    ResidentOperationFrameLocator, ResidentOperationFrameRequest, ResidentOperationFrameResponse,
};
use pale_resident::{
    ResidentConnectionAuthentication, ResidentHostError, ResidentHostTurn, ResidentLocalHost,
};
use std::{
    num::NonZeroUsize,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

fn started_locator(response: &[u8]) -> ResidentOperationFrameLocator {
    let ResidentOperationFrameResponse::Started { records } =
        serde_json::from_slice(response).unwrap()
    else {
        panic!("only the final authenticated application request may start work")
    };
    let value: serde_json::Value =
        serde_json::from_str(records.operation_handle_canonical()).unwrap();
    ResidentOperationFrameLocator::new(
        records.operation_handle_record_ref().to_owned(),
        value["canonical_operation_handle_sha256"]
            .as_str()
            .unwrap()
            .to_owned(),
    )
}

fn reject_peer(
    host: &mut ResidentLocalHost<'_, '_>,
    path: &Path,
    request: &[u8],
    mut caller: FixtureSigner,
    resident: [u8; 32],
    cause: LocalAuthenticationError,
) {
    std::thread::scope(|scope| {
        let client = scope.spawn(|| {
            let mut carrier = UnixSocketCarrier::connect(path, limits(), later()).unwrap();
            exchange_authenticated(
                &mut carrier,
                &mut caller,
                resident,
                &OsEntropy,
                request,
                later(),
            )
            .unwrap_err()
        });
        let until = later();
        let mut refused = false;
        while Instant::now() < until && !refused {
            match host.run_one(Some(deadlines()), true) {
                Err(ResidentHostError::Authentication(error)) => {
                    assert_eq!(error, cause);
                    refused = true;
                }
                Ok(ResidentHostTurn::Work(ResidentWorkTurnOutcome::Frame(_))) => {
                    panic!("unproved peer reached application")
                }
                Ok(_) => std::thread::yield_now(),
                other => panic!("unexpected peer refusal: {other:?}"),
            }
        }
        assert!(refused);
        assert_eq!(
            client.join().unwrap().request_delivery(),
            LocalRequestDelivery::NotSent
        );
    });
}

#[test]
fn plain_frames_unknown_callers_and_wrong_server_keys_never_reach_application() {
    with_in_memory_invoke_conformance_frontier_for_peer(
        observed_peer(),
        |service, requests| {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("authenticated.sock");
            let mut signer = FixtureSigner::resident();
            let mut host = ResidentLocalHost::bind(
                service,
                &path,
                limits(),
                NonZeroUsize::new(2).unwrap(),
                ResidentConnectionAuthentication::new(&mut signer, &OsEntropy, &selected_caller),
            )
            .unwrap();
            let request = serde_json::to_vec(&ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: requests.start_signed_common_request().to_owned(),
            })
            .unwrap();
            let mut plain = UnixSocketCarrier::connect(&path, limits(), later()).unwrap();
            plain.send_frame(&request, later()).unwrap();
            assert!(matches!(
                host.run_one(Some(deadlines()), true),
                Err(ResidentHostError::Authentication(
                    LocalAuthenticationError::Frame
                ))
            ));
            assert!(plain.receive_frame(later()).is_err());
            reject_peer(
                &mut host,
                &path,
                &request,
                FixtureSigner(SigningKey::from_bytes(&[43; 32])),
                FixtureSigner::resident_key(),
                LocalAuthenticationError::Peer,
            );
            reject_peer(
                &mut host,
                &path,
                &request,
                FixtureSigner::caller(),
                SigningKey::from_bytes(&[47; 32]).verifying_key().to_bytes(),
                LocalAuthenticationError::Carrier,
            );
            (started_locator(&exchange(&mut host, &path, &request)), ())
        },
        |_, _, _, ()| {},
    )
    .unwrap();
}

struct PausedRequest {
    carrier: UnixSocketCarrier,
    sends: usize,
    proved_server: mpsc::SyncSender<()>,
    resume: mpsc::Receiver<()>,
}

impl LocalIpcFramePort for PausedRequest {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        self.carrier.peer_credentials()
    }
    fn receive_frame(&mut self, deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.carrier.receive_frame(deadline)
    }
    fn send_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        self.sends += 1;
        if self.sends == 2 {
            self.proved_server.send(()).unwrap();
            self.resume
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap();
        }
        self.carrier.send_frame(payload, deadline)
    }
}

#[test]
fn quiet_challenged_client_allows_recovery_and_other_clients_and_rechecks_revocation() {
    with_in_memory_invoke_conformance_frontier_for_peer(
        observed_peer(),
        |service, requests| {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("authenticated.sock");
            let mut signer = FixtureSigner::resident();
            let selected = AtomicBool::new(true);
            let select = |key: &[u8; 32]| selected.load(Ordering::SeqCst) && selected_caller(key);
            let mut host = ResidentLocalHost::bind(
                service,
                &path,
                limits(),
                NonZeroUsize::new(2).unwrap(),
                ResidentConnectionAuthentication::new(&mut signer, &OsEntropy, &select),
            )
            .unwrap();
            let request = serde_json::to_vec(&ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: requests.start_signed_common_request().to_owned(),
            })
            .unwrap();
            std::thread::scope(|scope| {
                let (proved_tx, proved_rx) = mpsc::sync_channel(1);
                let (resume_tx, resume_rx) = mpsc::sync_channel(1);
                let client = scope.spawn(|| {
                    let mut carrier = PausedRequest {
                        carrier: UnixSocketCarrier::connect(&path, limits(), later()).unwrap(),
                        sends: 0,
                        proved_server: proved_tx,
                        resume: resume_rx,
                    };
                    exchange_authenticated(
                        &mut carrier,
                        &mut FixtureSigner::caller(),
                        FixtureSigner::resident_key(),
                        &OsEntropy,
                        &request,
                        later(),
                    )
                    .unwrap_err()
                });
                let until = later();
                while proved_rx.try_recv().is_err() {
                    assert!(Instant::now() < until);
                    assert!(!matches!(
                        host.run_one(Some(deadlines()), true).unwrap(),
                        ResidentHostTurn::Work(ResidentWorkTurnOutcome::Frame(_))
                    ));
                    std::thread::yield_now();
                }
                assert!(matches!(
                    host.run_one(None, true).unwrap(),
                    ResidentHostTurn::Work(
                        ResidentWorkTurnOutcome::RecoveryAdvanced
                            | ResidentWorkTurnOutcome::RecoveryDeferred
                    )
                ));
                let invalid = serde_json::to_vec(&ResidentOperationFrameRequest::StartInvoke {
                    signed_common_request: "{}".into(),
                })
                .unwrap();
                let response = exchange(&mut host, &path, &invalid);
                assert!(matches!(
                    serde_json::from_slice::<ResidentOperationFrameResponse>(&response).unwrap(),
                    ResidentOperationFrameResponse::Rejected { .. }
                ));
                selected.store(false, Ordering::SeqCst);
                resume_tx.send(()).unwrap();
                let until = later();
                loop {
                    assert!(Instant::now() < until);
                    match host.run_one(None, true) {
                        Err(ResidentHostError::Authentication(LocalAuthenticationError::Peer)) => {
                            break
                        }
                        Ok(ResidentHostTurn::Work(ResidentWorkTurnOutcome::Frame(_))) => {
                            panic!("revoked caller reached application")
                        }
                        Ok(_) => std::thread::sleep(Duration::from_millis(1)),
                        other => panic!("unexpected revocation outcome: {other:?}"),
                    }
                }
                assert_eq!(
                    client.join().unwrap().request_delivery(),
                    LocalRequestDelivery::Uncertain
                );
            });
            selected.store(true, Ordering::SeqCst);
            (started_locator(&exchange(&mut host, &path, &request)), ())
        },
        |_, _, _, ()| {},
    )
    .unwrap();
}
