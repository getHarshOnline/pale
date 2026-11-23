//! Test-only TLS forwarding faults around real native etcd transactions.

use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    ServerConfig, ServerConnection, StreamOwned,
};

use super::Quorum;

#[derive(Clone, Copy)]
pub(crate) enum Fault {
    DropTransactionReply,
    FenceReadResponses,
}

pub(crate) struct Proxy {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    owner: Option<JoinHandle<()>>,
    pub(crate) transactions: Arc<AtomicUsize>,
}

impl Proxy {
    pub(crate) fn start(quorum: &Quorum, fault: Fault) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let certificates = CertificateDer::pem_slice_iter(
            &fs::read(quorum.root.path().join("server.crt")).unwrap(),
        )
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        let key = PrivateKeyDer::from_pem_file(quorum.root.path().join("server.key")).unwrap();
        let config = Arc::new(
            ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::aws_lc_rs::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(certificates, key)
            .unwrap(),
        );
        let client = quorum.client.clone();
        let endpoint = quorum.endpoint(0);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let transactions = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&transactions);
        let fence = Arc::new((Mutex::new(0), Condvar::new()));
        let owner = thread::spawn(move || {
            let mut workers = Vec::new();
            while !stopped.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((socket, _)) => {
                        if stopped.load(Ordering::SeqCst) {
                            break;
                        }
                        let config = Arc::clone(&config);
                        let client = client.clone();
                        let endpoint = endpoint.clone();
                        let count = Arc::clone(&count);
                        let fence = Arc::clone(&fence);
                        workers.push(thread::spawn(move|| {
                            socket.set_nonblocking(false).unwrap();socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();socket.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
                            let mut stream=StreamOwned::new(ServerConnection::new(config).unwrap(),socket);
                            let (path,token,body)=request(&mut stream);
                            if path=="/v3/kv/txn" {count.fetch_add(1,Ordering::SeqCst);}
                            let response=client.post(format!("{endpoint}{path}")).header(reqwest::header::AUTHORIZATION,token)
                                .header(reqwest::header::CONTENT_TYPE,"application/json").body(body).send().unwrap();
                            let status=response.status().as_u16();let bytes=response.bytes().unwrap();
                            if path=="/v3/kv/range" && matches!(fault,Fault::FenceReadResponses) {
                                let (lock,condition)=&*fence;let mut arrivals=lock.lock().unwrap();*arrivals+=1;condition.notify_all();
                                let (arrivals,_)=condition.wait_timeout_while(arrivals,Duration::from_secs(5),|v|*v<2).unwrap();
                                assert!(*arrivals>=2,"both reads must observe the predecessor before either CAS");
                            }
                            if path=="/v3/kv/txn" && matches!(fault,Fault::DropTransactionReply) {return;}
                            write!(stream,"HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).unwrap();
                            stream.write_all(&bytes).unwrap();stream.flush().unwrap();
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("native fault listener: {e}"),
                }
            }
            for worker in workers {
                let _ = worker.join();
            }
        });
        Self {
            address,
            stop,
            owner: Some(owner),
            transactions,
        }
    }

    pub(crate) fn endpoint(&self) -> String {
        format!("https://{}", self.address)
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(owner) = self.owner.take() {
            let _ = owner.join();
        }
    }
}

fn request(stream: &mut impl Read) -> (String, String, Vec<u8>) {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        assert!(head.len() < 8192);
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        head.push(byte[0]);
    }
    let head = String::from_utf8(head).unwrap();
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap();
    let mut words = first.split_whitespace();
    assert_eq!(words.next(), Some("POST"));
    let path = words.next().unwrap().to_owned();
    assert!(matches!(path.as_str(), "/v3/kv/range" | "/v3/kv/txn"));
    let mut size = None;
    let mut token = None;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            match name.to_ascii_lowercase().as_str() {
                "content-length" => size = Some(value.trim().parse::<usize>().unwrap()),
                "authorization" => token = Some(value.trim().to_owned()),
                _ => {}
            }
        }
    }
    let size = size.unwrap();
    assert!(size <= 65536);
    let mut body = vec![0; size];
    stream.read_exact(&mut body).unwrap();
    (path, token.unwrap(), body)
}
