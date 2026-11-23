//! One owned, bounded loopback TLS fixture; not a reusable HTTP server.

use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig, ServerConnection, StreamOwned,
};

pub(super) const CA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/a2a-tls/ca.der"
));
pub(super) const OTHER_CA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/a2a-tls/unrelated-ca.der"
));
const LEAF: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/a2a-tls/leaf.der"
));
const KEY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/a2a-tls/leaf-key.der"
));
const WRONG_LEAF: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/a2a-tls/wrong-host.der"
));
const WRONG_KEY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/a2a-tls/wrong-host-key.der"
));

#[derive(Clone)]
pub(super) enum Reply {
    Bytes(Vec<u8>),
    LostAfterBody,
    Stall,
}

#[derive(Clone)]
pub(super) struct CapturedRequest {
    pub head: String,
    pub body: Vec<u8>,
}

pub(super) struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    accepts: Arc<AtomicUsize>,
    responses: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    thread: Option<JoinHandle<io::Result<()>>>,
}

impl Server {
    pub(super) fn start(reply: Reply, wrong_host: bool) -> Self {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let (certificate, key) = if wrong_host {
            (WRONG_LEAF, WRONG_KEY)
        } else {
            (LEAF, KEY)
        };
        let config = ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(certificate.to_vec())],
            PrivateKeyDer::try_from(key.to_vec()).unwrap(),
        )
        .unwrap();
        let config = Arc::new(config);
        let stop = Arc::new(AtomicBool::new(false));
        let accepts = Arc::new(AtomicUsize::new(0));
        let responses = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (stopped, accepted, captured) = (stop.clone(), accepts.clone(), requests.clone());
        let written = responses.clone();
        let thread = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                let socket = match listener.accept() {
                    Ok((socket, _)) => socket,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::park_timeout(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                if accepted.fetch_add(1, Ordering::AcqRel) >= 8 {
                    return Err(io::Error::other("fixture connection ceiling"));
                }
                // StreamOwned uses synchronous I/O; only the listener polls.
                socket.set_nonblocking(false)?;
                socket.set_read_timeout(Some(Duration::from_secs(2)))?;
                socket.set_write_timeout(Some(Duration::from_secs(2)))?;
                let mut stream = StreamOwned::new(
                    ServerConnection::new(config.clone()).map_err(io::Error::other)?,
                    socket,
                );
                let request = match read_request(&mut stream) {
                    Ok(request) => request,
                    Err(error) => {
                        // Only this synthetic peer logs its own bounded handshake error.
                        // Production transport errors remain closed and redacted.
                        eprintln!(
                            "test TLS peer before HTTP: {}",
                            error.to_string().chars().take(256).collect::<String>()
                        );
                        continue;
                    }
                };
                captured.lock().unwrap().push(request);
                match &reply {
                    Reply::Bytes(bytes) => {
                        if stream
                            .write_all(bytes)
                            .and_then(|()| stream.flush())
                            .is_ok()
                        {
                            written.fetch_add(1, Ordering::Release);
                        }
                    }
                    Reply::LostAfterBody => {}
                    Reply::Stall => {
                        stream
                            .sock
                            .set_read_timeout(Some(Duration::from_millis(20)))?;
                        let mut byte = [0];
                        while !stopped.load(Ordering::Acquire) {
                            match stream.read(&mut byte) {
                                Ok(0) => break,
                                Ok(_) => {
                                    return Err(io::Error::other("unexpected extra request bytes"));
                                }
                                Err(error)
                                    if matches!(
                                        error.kind(),
                                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                                    ) => {}
                                Err(_) => break,
                            }
                        }
                    }
                }
            }
            Ok(())
        });
        Self {
            port,
            stop,
            accepts,
            responses,
            requests,
            thread: Some(thread),
        }
    }

    pub(super) fn endpoint(&self) -> String {
        format!("https://127.0.0.1:{}/rpc?selected=one", self.port)
    }
    pub(super) fn requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().unwrap().clone()
    }
    pub(super) fn accepted(&self) -> usize {
        self.accepts.load(Ordering::Acquire)
    }
    pub(super) fn responses(&self) -> usize {
        self.responses.load(Ordering::Acquire)
    }
    pub(super) fn finish(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap().unwrap();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn read_request(
    stream: &mut StreamOwned<ServerConnection, TcpStream>,
) -> io::Result<CapturedRequest> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() == 8192 {
            return Err(io::Error::other("fixture header ceiling"));
        }
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        bytes.push(byte[0]);
    }
    let head = String::from_utf8(bytes).map_err(io::Error::other)?;
    let mut lengths = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"));
    let length = lengths
        .next()
        .ok_or_else(|| io::Error::other("missing request length"))?
        .1
        .trim()
        .parse::<usize>()
        .map_err(io::Error::other)?;
    if lengths.next().is_some() || length > 4096 {
        return Err(io::Error::other("fixture body ceiling"));
    }
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    Ok(CapturedRequest { head, body })
}
