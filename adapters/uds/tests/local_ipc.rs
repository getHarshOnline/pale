//! Behavioral proof for the bounded Unix-domain socket carrier.

#[path = "local_ipc/authentication.rs"]
mod authentication;

use std::io::Write;
use std::num::NonZeroU32;
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::{Duration, Instant};

use pale_adapter_uds::{UnixSocketCarrier, UnixSocketListener};
use pale_ports::{
    LocalIpcCarrierError, LocalIpcCarrierOperation, LocalIpcFrameLimits, LocalIpcFramePort,
    LocalIpcFrameSection,
};

fn limits(max_payload_bytes: u32) -> LocalIpcFrameLimits {
    LocalIpcFrameLimits::new(NonZeroU32::new(max_payload_bytes).expect("positive fixture limit"))
}

fn later() -> Instant {
    Instant::now() + Duration::from_secs(2)
}

#[test]
fn nonblocking_accept_and_readiness_preserve_the_unread_frame() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ready.sock");
    let listener = UnixSocketListener::bind(&path, limits(64)).unwrap();
    assert!(listener.try_accept().unwrap().is_none());
    let mut client = UnixSocketCarrier::connect(&path, limits(64), later()).unwrap();
    let mut server = listener.try_accept().unwrap().expect("queued native peer");
    assert!(!server.is_readable().unwrap());
    client.send_frame(b"original bytes", later()).unwrap();
    assert!(server.is_readable().unwrap());
    assert!(server.is_readable().unwrap());
    assert_eq!(&*server.receive_frame(later()).unwrap(), b"original bytes");
    assert!(!server.is_readable().unwrap());
    drop(client);
    assert!(server.is_readable().unwrap());
    assert!(matches!(
        server.receive_frame(later()),
        Err(LocalIpcCarrierError::EndOfStream)
    ));
    assert!(matches!(
        server.is_readable(),
        Err(LocalIpcCarrierError::ConnectionUnusable)
    ));
}

#[test]
fn round_trip_preserves_opaque_bytes_and_observes_kernel_peer_coordinates() {
    let directory = tempfile::tempdir().expect("temporary socket directory");
    let path = directory.path().join("resident.sock");
    let listener = UnixSocketListener::bind(&path, limits(64)).expect("listener");

    let server = thread::spawn(move || {
        let mut connection = listener.accept(later()).expect("accept");
        let peer = connection.peer_credentials().expect("server peer");
        let request = connection.receive_frame(later()).expect("request");
        assert_eq!(&*request, &[0xff, 0x00, 0x80, b'J']);
        connection.send_frame(&request, later()).expect("response");
        peer
    });

    let mut client = UnixSocketCarrier::connect(&path, limits(64), later()).expect("connect");
    let client_peer = client.peer_credentials().expect("client peer");
    client
        .send_frame(&[0xff, 0x00, 0x80, b'J'], later())
        .expect("send");
    assert_eq!(
        &*client.receive_frame(later()).expect("receive"),
        &[0xff, 0x00, 0x80, b'J']
    );

    let server_peer = server.join().expect("server thread");
    assert_eq!(server_peer.process_id(), Some(std::process::id()));
    assert_eq!(client_peer.process_id(), Some(std::process::id()));
    assert_eq!(server_peer.user_id(), nix::unistd::getuid().as_raw());
    assert_eq!(server_peer.group_id(), nix::unistd::getgid().as_raw());
}

#[test]
fn accept_honors_an_absolute_deadline() {
    let directory = tempfile::tempdir().expect("temporary socket directory");
    let path = directory.path().join("resident.sock");
    let listener = UnixSocketListener::bind(&path, limits(8)).expect("listener");

    let error = match listener.accept(Instant::now() + Duration::from_millis(20)) {
        Ok(_) => panic!("accept unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        LocalIpcCarrierError::DeadlineElapsed {
            operation: LocalIpcCarrierOperation::Accept,
            frame_bytes_transferred: 0,
        }
    ));
}

#[test]
fn receive_distinguishes_clean_eof_from_a_partial_length_prefix() {
    let directory = tempfile::tempdir().expect("temporary socket directory");
    let clean_path = directory.path().join("clean.sock");
    let clean_listener = UnixSocketListener::bind(&clean_path, limits(8)).expect("listener");
    let clean_peer = UnixStream::connect(&clean_path).expect("raw peer");
    let mut clean_connection = clean_listener.accept(later()).expect("accept");
    drop(clean_peer);
    assert!(matches!(
        clean_connection.receive_frame(later()),
        Err(LocalIpcCarrierError::EndOfStream)
    ));

    let partial_path = directory.path().join("partial.sock");
    let partial_listener = UnixSocketListener::bind(&partial_path, limits(8)).expect("listener");
    let mut partial_peer = UnixStream::connect(&partial_path).expect("raw peer");
    partial_peer.write_all(&[0, 0]).expect("partial prefix");
    partial_peer
        .shutdown(std::net::Shutdown::Write)
        .expect("shutdown");
    let mut partial_connection = partial_listener.accept(later()).expect("accept");
    assert!(matches!(
        partial_connection.receive_frame(later()),
        Err(LocalIpcCarrierError::TruncatedFrame {
            section: LocalIpcFrameSection::LengthPrefix,
            expected_bytes: 4,
            received_bytes: 2,
        })
    ));
    assert!(matches!(
        partial_connection.receive_frame(later()),
        Err(LocalIpcCarrierError::ConnectionUnusable)
    ));
}

#[test]
fn receive_reports_a_truncated_payload() {
    let directory = tempfile::tempdir().expect("temporary socket directory");
    let path = directory.path().join("resident.sock");
    let listener = UnixSocketListener::bind(&path, limits(8)).expect("listener");
    let mut peer = UnixStream::connect(&path).expect("raw peer");
    peer.write_all(&5_u32.to_be_bytes()).expect("length");
    peer.write_all(b"ab").expect("partial payload");
    peer.shutdown(std::net::Shutdown::Write).expect("shutdown");
    let mut connection = listener.accept(later()).expect("accept");

    assert!(matches!(
        connection.receive_frame(later()),
        Err(LocalIpcCarrierError::TruncatedFrame {
            section: LocalIpcFrameSection::Payload,
            expected_bytes: 5,
            received_bytes: 2,
        })
    ));
    assert!(matches!(
        connection.receive_frame(later()),
        Err(LocalIpcCarrierError::ConnectionUnusable)
    ));
}

#[test]
fn receive_rejects_an_oversized_declaration_before_reading_payload() {
    let directory = tempfile::tempdir().expect("temporary socket directory");
    let path = directory.path().join("resident.sock");
    let listener = UnixSocketListener::bind(&path, limits(4)).expect("listener");
    let mut peer = UnixStream::connect(&path).expect("raw peer");
    peer.write_all(&5_u32.to_be_bytes()).expect("length");
    let mut connection = listener.accept(later()).expect("accept");

    assert!(matches!(
        connection.receive_frame(later()),
        Err(LocalIpcCarrierError::FrameTooLarge {
            operation: LocalIpcCarrierOperation::ReceiveFrame,
            declared_bytes: 5,
            limit_bytes: 4,
        })
    ));
}

#[test]
fn receive_and_send_apply_deadlines_without_ambient_defaults() {
    let directory = tempfile::tempdir().expect("temporary socket directory");
    let path = directory.path().join("resident.sock");
    let listener = UnixSocketListener::bind(&path, limits(8)).expect("listener");
    let mut client = UnixSocketCarrier::connect(&path, limits(8), later()).expect("connect");
    let mut server = listener.accept(later()).expect("accept");

    assert!(matches!(
        server.receive_frame(Instant::now() + Duration::from_millis(20)),
        Err(LocalIpcCarrierError::DeadlineElapsed {
            operation: LocalIpcCarrierOperation::ReceiveFrame,
            frame_bytes_transferred: 0,
        })
    ));
    assert!(matches!(
        client.send_frame(b"x", Instant::now()),
        Err(LocalIpcCarrierError::DeadlineElapsed {
            operation: LocalIpcCarrierOperation::SendFrame,
            frame_bytes_transferred: 0,
        })
    ));
    client.send_frame(b"x", later()).expect("retry send");
    assert_eq!(
        &*server.receive_frame(later()).expect("retry receive"),
        b"x"
    );
}

#[test]
fn send_rejects_oversize_before_transmission() {
    let directory = tempfile::tempdir().expect("temporary socket directory");
    let path = directory.path().join("resident.sock");
    let listener = UnixSocketListener::bind(&path, limits(4)).expect("listener");
    let mut client = UnixSocketCarrier::connect(&path, limits(4), later()).expect("connect");
    let mut server = listener.accept(later()).expect("accept");

    assert!(matches!(
        client.send_frame(b"12345", later()),
        Err(LocalIpcCarrierError::FrameTooLarge {
            operation: LocalIpcCarrierOperation::SendFrame,
            declared_bytes: 5,
            limit_bytes: 4,
        })
    ));
    client.send_frame(b"1234", later()).expect("bounded retry");
    assert_eq!(
        &*server
            .receive_frame(later())
            .expect("receive bounded retry"),
        b"1234"
    );
}

#[test]
fn connection_errors_preserve_a_typed_source_without_echoing_the_path() {
    let directory = tempfile::tempdir().expect("temporary socket directory");
    let path = directory.path().join("private-endpoint-name.sock");
    let error = match UnixSocketCarrier::connect(&path, limits(8), later()) {
        Ok(_) => panic!("missing endpoint unexpectedly accepted a connection"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        LocalIpcCarrierError::Io {
            operation: LocalIpcCarrierOperation::Connect,
            ..
        }
    ));
    assert!(!error.to_string().contains("private-endpoint-name"));
    assert!(std::error::Error::source(&error).is_some());
}
