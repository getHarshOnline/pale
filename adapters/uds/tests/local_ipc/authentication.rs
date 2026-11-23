//! Authentication uses real sockets and signatures, with isolated fixture keys.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ed25519_dalek::{Signer as _, SigningKey};
use pale_adapter_uds::{
    accept_authenticated_request, exchange_authenticated, AuthenticatedLocalClient,
    LocalAuthenticatedExchangeError, LocalAuthenticationError, LocalRequestDelivery,
    UnixSocketCarrier, UnixSocketListener, LOCAL_AUTHENTICATED_FRAME_BYTE_LIMIT,
    LOCAL_AUTHENTICATED_PAYLOAD_BYTE_LIMIT,
};
use pale_ports::{
    CryptographicRandomSourceError, CryptographicRandomSourcePort, LocalConnectionSignerPort,
    LocalConnectionSigningError, LocalIpcCarrierError, LocalIpcFramePort, LocalIpcPeerCredentials,
};

struct FixtureSigner(SigningKey, bool, usize);

struct NoIo;

impl LocalIpcFramePort for NoIo {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        panic!("rejected input must not touch the carrier")
    }

    fn send_frame(&mut self, _: &[u8], _: Instant) -> Result<(), LocalIpcCarrierError> {
        panic!("rejected input must not touch the carrier")
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        panic!("rejected input must not touch the carrier")
    }
}

struct UnavailableEntropy;

impl CryptographicRandomSourcePort for UnavailableEntropy {
    fn fill_bytes(&self, _: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        Err(CryptographicRandomSourceError::unavailable())
    }
}

#[test]
fn missing_entropy_expired_deadline_and_bad_bounds_send_no_request() {
    let mut signer = FixtureSigner(SigningKey::from_bytes(&[17; 32]), false, 0);
    let resident_key = SigningKey::from_bytes(&[29; 32]).verifying_key().to_bytes();
    let deadline = Instant::now() + Duration::from_secs(2);
    let error = exchange_authenticated(
        &mut NoIo,
        &mut signer,
        resident_key,
        &UnavailableEntropy,
        b"request",
        deadline,
    )
    .unwrap_err();
    assert_eq!(error.cause(), LocalAuthenticationError::Entropy);
    assert_eq!(error.request_delivery(), LocalRequestDelivery::NotSent);

    let error = exchange_authenticated(
        &mut NoIo,
        &mut signer,
        resident_key,
        &OsEntropy,
        b"request",
        Instant::now(),
    )
    .unwrap_err();
    assert_eq!(error.cause(), LocalAuthenticationError::Deadline);
    assert_eq!(error.request_delivery(), LocalRequestDelivery::NotSent);

    for payload in [
        Vec::new(),
        vec![0; LOCAL_AUTHENTICATED_PAYLOAD_BYTE_LIMIT + 1],
    ] {
        let error = exchange_authenticated(
            &mut NoIo,
            &mut signer,
            resident_key,
            &OsEntropy,
            &payload,
            deadline,
        )
        .unwrap_err();
        assert_eq!(error.cause(), LocalAuthenticationError::Frame);
        assert_eq!(error.request_delivery(), LocalRequestDelivery::NotSent);
    }
}

struct HelloOnly(Vec<u8>);

impl LocalIpcFramePort for HelloOnly {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        panic!("this check supplies only an untrusted hello")
    }

    fn send_frame(&mut self, _: &[u8], _: Instant) -> Result<(), LocalIpcCarrierError> {
        panic!("failed authentication preparation must not send a challenge")
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        assert!(!self.0.is_empty(), "no application request may be read");
        Ok(std::mem::take(&mut self.0).into_boxed_slice())
    }
}

struct UnavailableSigner([u8; 32]);

impl LocalConnectionSignerPort for UnavailableSigner {
    fn public_key(&self) -> [u8; 32] {
        self.0
    }

    fn sign(&mut self, _: &[u8], _: Instant) -> Result<[u8; 64], LocalConnectionSigningError> {
        Err(LocalConnectionSigningError)
    }
}

#[test]
fn server_cannot_continue_without_fresh_challenge_and_selected_signer() {
    let prior = exchange(Interference::None, Interference::None, true, false);
    prior.server.unwrap();
    let hello = &prior.client_frames[0];
    let mut signer = FixtureSigner(SigningKey::from_bytes(&[29; 32]), false, 0);
    let deadline = Instant::now() + Duration::from_secs(2);
    let error = accept_authenticated_request(
        &mut HelloOnly(hello.clone()),
        &mut signer,
        &UnavailableEntropy,
        deadline,
        |_| true,
    )
    .err()
    .unwrap();
    assert_eq!(error, LocalAuthenticationError::Entropy);
    let error = accept_authenticated_request(
        &mut HelloOnly(hello.clone()),
        &mut UnavailableSigner(signer.public_key()),
        &OsEntropy,
        deadline,
        |_| true,
    )
    .err()
    .unwrap();
    assert_eq!(error, LocalAuthenticationError::Signing);
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
        self.2 += 1;
        if self.1 {
            std::thread::sleep(
                deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(2),
            );
        }
        Ok(self.0.sign(message).to_bytes())
    }
}

struct OsEntropy;

impl CryptographicRandomSourcePort for OsEntropy {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        getrandom::fill(destination).map_err(|_| CryptographicRandomSourceError::unavailable())
    }
}

#[derive(Default)]
enum Interference {
    #[default]
    None,
    AlterSecondFrame,
    ReplaySecondFrame(Vec<u8>),
}

struct RecordingCarrier {
    inner: UnixSocketCarrier,
    writes: Arc<Mutex<Vec<Vec<u8>>>>,
    interference: Interference,
}

impl LocalIpcFramePort for RecordingCarrier {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        self.inner.peer_credentials()
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        let mut writes = self.writes.lock().unwrap();
        let mut payload = payload.to_vec();
        if writes.len() == 1 {
            match &self.interference {
                Interference::None => {}
                Interference::AlterSecondFrame => *payload.last_mut().unwrap() ^= 1,
                Interference::ReplaySecondFrame(replay) => payload.clone_from(replay),
            }
        }
        writes.push(payload.clone());
        drop(writes);
        self.inner.send_frame(&payload, deadline)
    }

    fn receive_frame(&mut self, deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.inner.receive_frame(deadline)
    }
}

struct ObservedExchange {
    client: Result<Box<[u8]>, LocalAuthenticatedExchangeError>,
    server: Result<(), LocalAuthenticationError>,
    client_frames: Vec<Vec<u8>>,
    server_frames: Vec<Vec<u8>>,
    admitted: Vec<Vec<u8>>,
    client_sign_calls: usize,
}

fn exchange(
    client_interference: Interference,
    server_interference: Interference,
    select_client: bool,
    wrong_resident: bool,
) -> ObservedExchange {
    exchange_with_expiry(
        client_interference,
        server_interference,
        select_client,
        wrong_resident,
        false,
    )
}

fn exchange_with_expiry(
    client_interference: Interference,
    server_interference: Interference,
    select_client: bool,
    wrong_resident: bool,
    expire_caller: bool,
) -> ObservedExchange {
    exchange_with_client(
        client_interference,
        server_interference,
        select_client,
        wrong_resident,
        expire_caller,
        ClientStyle::Combined,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClientStyle {
    Combined,
    Split,
    SubstituteSigner,
    ExpireConnection,
}

fn exchange_with_client(
    client_interference: Interference,
    server_interference: Interference,
    select_client: bool,
    wrong_resident: bool,
    expire_caller: bool,
    style: ClientStyle,
) -> ObservedExchange {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("authenticated.sock");
    let limits = super::limits(LOCAL_AUTHENTICATED_FRAME_BYTE_LIMIT);
    let listener = UnixSocketListener::bind(&path, limits).unwrap();
    let mut client_signer = FixtureSigner(SigningKey::from_bytes(&[17; 32]), false, 0);
    client_signer.1 = expire_caller;
    let caller_key = client_signer.public_key();
    let mut server_signer = FixtureSigner(SigningKey::from_bytes(&[29; 32]), false, 0);
    let resident_key = if wrong_resident {
        SigningKey::from_bytes(&[47; 32]).verifying_key().to_bytes()
    } else {
        server_signer.public_key()
    };
    let deadline = Instant::now()
        + if expire_caller || style == ClientStyle::ExpireConnection {
            Duration::from_millis(250)
        } else {
            Duration::from_secs(3)
        };
    let server_frames = Arc::new(Mutex::new(Vec::new()));
    let admitted = Arc::new(Mutex::new(Vec::new()));
    let server_writes = Arc::clone(&server_frames);
    let server_admitted = Arc::clone(&admitted);
    let server = std::thread::spawn(move || {
        let inner = listener.accept(deadline).unwrap();
        let mut carrier = RecordingCarrier {
            inner,
            writes: server_writes,
            interference: server_interference,
        };
        let request = accept_authenticated_request(
            &mut carrier,
            &mut server_signer,
            &OsEntropy,
            deadline,
            |key| select_client && key == &caller_key,
        )?;
        assert_eq!(request.caller_key(), &caller_key);
        server_admitted
            .lock()
            .unwrap()
            .push(request.payload().to_vec());
        request.respond(
            &mut carrier,
            &mut server_signer,
            b"bounded application response",
        )
    });
    let client_frames = Arc::new(Mutex::new(Vec::new()));
    let mut carrier = RecordingCarrier {
        inner: UnixSocketCarrier::connect(&path, limits, deadline).unwrap(),
        writes: Arc::clone(&client_frames),
        interference: client_interference,
    };
    let client = if style == ClientStyle::Combined {
        let result = exchange_authenticated(
            &mut carrier,
            &mut client_signer,
            resident_key,
            &OsEntropy,
            b"private native event",
            deadline,
        );
        drop(carrier);
        result
    } else {
        let client = AuthenticatedLocalClient::authenticate(
            carrier,
            caller_key,
            resident_key,
            &OsEntropy,
            deadline,
        )
        .unwrap();
        assert!(
            admitted.lock().unwrap().is_empty(),
            "authentication sends no application payload"
        );
        assert_eq!(client_frames.lock().unwrap().len(), 1);
        if style == ClientStyle::SubstituteSigner {
            client_signer.0 = SigningKey::from_bytes(&[31; 32]);
        }
        if style == ClientStyle::ExpireConnection {
            std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
        }
        client.exchange(
            &mut client_signer,
            b"private native event",
            Instant::now() + Duration::from_secs(3),
        )
    };
    let server = server.join().unwrap();
    ObservedExchange {
        client,
        server,
        client_sign_calls: client_signer.2,
        client_frames: Arc::try_unwrap(client_frames)
            .unwrap()
            .into_inner()
            .unwrap(),
        server_frames: Arc::try_unwrap(server_frames)
            .unwrap()
            .into_inner()
            .unwrap(),
        admitted: Arc::try_unwrap(admitted).unwrap().into_inner().unwrap(),
    }
}

#[test]
fn pinned_peers_exchange_exact_bytes_over_real_socket() {
    let result = exchange(Interference::None, Interference::None, true, false);
    assert_eq!(&*result.client.unwrap(), b"bounded application response");
    result.server.unwrap();
    assert_eq!(result.admitted, [b"private native event".to_vec()]);
    assert_eq!(result.client_frames.len(), 2);
    assert_eq!(result.server_frames.len(), 2);
}

#[test]
fn split_client_authenticates_before_payload_and_cannot_change_signer_or_extend_deadline() {
    for style in [
        ClientStyle::Split,
        ClientStyle::SubstituteSigner,
        ClientStyle::ExpireConnection,
    ] {
        let result = exchange_with_client(
            Interference::None,
            Interference::None,
            true,
            false,
            false,
            style,
        );
        if style == ClientStyle::Split {
            assert_eq!(&*result.client.unwrap(), b"bounded application response");
            result.server.unwrap();
            assert_eq!(result.admitted, [b"private native event".to_vec()]);
        } else {
            let error = result.client.unwrap_err();
            assert_eq!(error.request_delivery(), LocalRequestDelivery::NotSent);
            assert_eq!(
                error.cause(),
                if style == ClientStyle::SubstituteSigner {
                    LocalAuthenticationError::Peer
                } else {
                    LocalAuthenticationError::Deadline
                }
            );
            assert!(result.server.is_err());
            assert!(result.admitted.is_empty());
            assert_eq!(result.client_frames.len(), 1);
            assert_eq!(result.client_sign_calls, 0);
        }
    }
}

#[test]
fn wrong_resident_never_receives_private_payload() {
    let result = exchange(Interference::None, Interference::None, true, true);
    assert_eq!(
        result.client.as_ref().unwrap_err().cause(),
        LocalAuthenticationError::Peer
    );
    assert_eq!(
        result.client.as_ref().unwrap_err().request_delivery(),
        LocalRequestDelivery::NotSent
    );
    assert_eq!(result.client_frames.len(), 1);
    assert!(result.admitted.is_empty());
    assert!(result.server.is_err());
}

#[test]
fn unknown_caller_never_reaches_application_or_signing_challenge() {
    let result = exchange(Interference::None, Interference::None, false, false);
    assert_eq!(result.server, Err(LocalAuthenticationError::Peer));
    assert_eq!(
        result.client.as_ref().unwrap_err().request_delivery(),
        LocalRequestDelivery::NotSent
    );
    assert_eq!(result.client_frames.len(), 1);
    assert!(result.server_frames.is_empty());
    assert!(result.admitted.is_empty());
}

#[test]
fn altered_request_is_rejected_before_application_admission() {
    let result = exchange(
        Interference::AlterSecondFrame,
        Interference::None,
        true,
        false,
    );
    assert_eq!(result.server, Err(LocalAuthenticationError::Peer));
    assert!(result.client.is_err());
    assert!(result.admitted.is_empty());
}

#[test]
fn altered_response_is_rejected_without_repeating_delivered_request() {
    let result = exchange(
        Interference::None,
        Interference::AlterSecondFrame,
        true,
        false,
    );
    assert_eq!(
        result.client.as_ref().unwrap_err().cause(),
        LocalAuthenticationError::Peer
    );
    assert_eq!(
        result.client.as_ref().unwrap_err().request_delivery(),
        LocalRequestDelivery::Uncertain
    );
    assert_eq!(result.admitted.len(), 1);
    assert_eq!(result.client_frames.len(), 2);
    result.server.unwrap();
}

#[test]
fn captured_request_cannot_enter_a_new_connection() {
    let prior = exchange(Interference::None, Interference::None, true, false);
    prior.server.unwrap();
    let result = exchange(
        Interference::ReplaySecondFrame(prior.client_frames[1].clone()),
        Interference::None,
        true,
        false,
    );
    assert_eq!(result.server, Err(LocalAuthenticationError::Peer));
    assert!(result.admitted.is_empty());
}

#[test]
fn captured_response_cannot_complete_another_exchange() {
    let prior = exchange(Interference::None, Interference::None, true, false);
    prior.server.unwrap();
    let result = exchange(
        Interference::None,
        Interference::ReplaySecondFrame(prior.server_frames[1].clone()),
        true,
        false,
    );
    assert_eq!(
        result.client.as_ref().unwrap_err().cause(),
        LocalAuthenticationError::Peer
    );
    assert_eq!(
        result.client.as_ref().unwrap_err().request_delivery(),
        LocalRequestDelivery::Uncertain
    );
    assert_eq!(result.admitted.len(), 1);
}

#[test]
fn expiry_after_signing_does_not_claim_request_delivery() {
    let result = exchange_with_expiry(Interference::None, Interference::None, true, false, true);
    let error = result.client.unwrap_err();
    assert_eq!(result.client_sign_calls, 1);
    assert_eq!(error.cause(), LocalAuthenticationError::Deadline);
    assert_eq!(error.request_delivery(), LocalRequestDelivery::NotSent);
    assert_eq!(result.client_frames.len(), 1);
    assert!(result.admitted.is_empty());
}

#[test]
fn pending_challenge_returns_to_the_loop_while_client_is_quiet() {
    let prior = exchange(Interference::None, Interference::None, true, false);
    prior.server.unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("quiet-auth.sock");
    let limits = super::limits(LOCAL_AUTHENTICATED_FRAME_BYTE_LIMIT);
    let listener = UnixSocketListener::bind(&path, limits).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut client = UnixSocketCarrier::connect(&path, limits, deadline).unwrap();
    let mut server = listener.accept(deadline).unwrap();
    client
        .send_frame(&prior.client_frames[0], deadline)
        .unwrap();
    let mut signer = FixtureSigner(SigningKey::from_bytes(&[29; 32]), false, 0);
    let pending = pale_adapter_uds::begin_authenticated_request(
        &mut server,
        &mut signer,
        &OsEntropy,
        deadline,
        |_| true,
    )
    .unwrap();
    assert!(!server.is_readable().unwrap());
    assert_eq!(client.receive_frame(deadline).unwrap().len(), 112);
    drop(pending);
}
